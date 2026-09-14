extends Node3D
## Monta a torre do nível.
##
## REGRA DE DESIGN CENTRAL: a energia é o relógio do jogo.
## O espaçamento entre estações de recarga é o que define a dificuldade —
## mais longe = mais tenso. Veja DESIGN.md.

signal nivel_pronto(altura_total: float)
signal drone_destruido(pontos: int)

const PONTOS_DRONE := 150

const CENA_RESGATE := preload("res://scenes/Resgate.tscn")
const CENA_DRONE := preload("res://scenes/Drone.tscn")
const CENA_HELICOPTERO := preload("res://scenes/Helicoptero.tscn")
const CENA_RECARGA := preload("res://scenes/EstacaoRecarga.tscn")

const LARGURA_CONTEINER := 6.0
const ALTURA_CONTEINER := 3.0
const COLUNAS := 7

## Modelos reais (CC0, Kenney "City Kit Industrial") substituem os blocos
## coloridos primitivos. O comprimento natural do modelo (~3.047) escalado
## pra caber na largura do vão do jogo (6.0) — ver assets/licencas/.
const CENAS_CONTEINER: Array[PackedScene] = [
	preload("res://assets/modelos/shipping-container-a.glb"),
	preload("res://assets/modelos/shipping-container-b.glb"),
	preload("res://assets/modelos/shipping-container-c.glb"),
]
## 3.046667 = comprimento cru da malha; 0.27 = escala interna já embutida
## no nó do modelo (achada inspecionando a cadeia de transform completa).
const ESCALA_CONTEINER := LARGURA_CONTEINER / (3.046667 * 0.27)

## O modelo não tem textura própria (geometria branca lisa — a cor que se vê
## vem só da luz da cena) — então dá pra tingir livremente sem perder detalhe.
## Paleta de cores reais de contêiner + duas variantes enferrujadas (tom
## terroso, mais ásperas/foscas) pra quebrar a monotonia visual da torre.
## `zona`: 0 = base da torre (industrial/enferrujado), 1 = corpo (mix atual),
## 2 = topo (tons frios/vívidos) — sinaliza progresso na subida sem precisar
## de UI: o jogador SENTE que está ficando mais alto pela cor ao redor.
const PALETA_CONTEINER := [
	{"cor": Color(0.12, 0.36, 0.56), "rugosidade": 0.75, "metalico": 0.2, "zona": 1},   # azul
	{"cor": Color(0.55, 0.15, 0.13), "rugosidade": 0.75, "metalico": 0.2, "zona": 0},   # vermelho
	{"cor": Color(0.17, 0.33, 0.22), "rugosidade": 0.75, "metalico": 0.2, "zona": 1},   # verde
	{"cor": Color(0.68, 0.52, 0.15), "rugosidade": 0.75, "metalico": 0.2, "zona": 0},   # amarelo mostarda
	{"cor": Color(0.15, 0.42, 0.43), "rugosidade": 0.75, "metalico": 0.2, "zona": 2},   # azul petróleo
	{"cor": Color(0.52, 0.30, 0.14), "rugosidade": 0.95, "metalico": 0.05, "zona": 0},  # enferrujado claro
	{"cor": Color(0.36, 0.23, 0.15), "rugosidade": 0.95, "metalico": 0.0, "zona": 0},   # enferrujado escuro
	{"cor": Color(0.55, 0.66, 0.72), "rugosidade": 0.6, "metalico": 0.3, "zona": 2},    # cinza-gelo (topo)
	{"cor": Color(0.18, 0.56, 0.58), "rugosidade": 0.65, "metalico": 0.25, "zona": 2},  # ciano (topo)
]

var _forma_conteiner: BoxShape3D
var _materiais_conteiner: Array[StandardMaterial3D] = []
var _indices_por_zona: Dictionary = {0: [], 1: [], 2: []}

var nivel_atual: int = 1
var altura_total: float = 0.0
var pos_inicial: Vector3 = Vector3.ZERO
var pos_helicoptero: Vector3 = Vector3.ZERO
var total_resgates: int = 0
var helicoptero_atual: Node3D = null


## Perfil de dificuldade de cada fase.
## recarga_cada: de quantos em quantos andares aparece uma estação.
##   Maior = postos mais distantes = mais tensão. É o botão principal
##   de dificuldade do jogo inteiro.
func perfil_da_fase(n: int) -> Dictionary:
	match n:
		1:
			# Ensina o básico. Recarga sobrando, quase sem inimigo.
			return {andares = 20, recarga_cada = 4, carga_estacao = 80.0,
					resgates = 3, chance_drone = 0.06, resistencia_conteiner = 1}
		2:
			# Aperta o espaçamento. Introduz drones de verdade.
			return {andares = 26, recarga_cada = 5, carga_estacao = 70.0,
					resgates = 4, chance_drone = 0.14, resistencia_conteiner = 1}
		3:
			# Contêineres mais duros: usar o laser passa a custar caro.
			return {andares = 32, recarga_cada = 6, carga_estacao = 65.0,
					resgates = 5, chance_drone = 0.20, resistencia_conteiner = 2}
		4:
			# Estações escassas. Aqui o jogador precisa planejar a rota.
			return {andares = 38, recarga_cada = 8, carga_estacao = 60.0,
					resgates = 6, chance_drone = 0.26, resistencia_conteiner = 2}
		_:
			# Fase 5+: escala contínua, com piso pra não virar impossível
			var extra: int = n - 4
			return {
				andares = 38 + extra * 6,
				recarga_cada = mini(10, 8 + extra / 2),
				carga_estacao = maxf(45.0, 60.0 - extra * 3.0),
				resgates = mini(9, 6 + extra / 2),
				chance_drone = minf(0.40, 0.26 + extra * 0.03),
				resistencia_conteiner = 3,
			}


func gerar(nivel: int) -> void:
	nivel_atual = nivel
	_preparar_recursos_conteiner()
	_limpar()

	var p := perfil_da_fase(nivel)
	var andares: int = p.andares
	altura_total = andares * ALTURA_CONTEINER

	var col_vao: int = COLUNAS / 2
	var andares_de_resgate := _sortear_andares(p.resgates, 4, andares - 3)

	for andar in range(andares):
		if andar > 2:
			col_vao = clampi(col_vao + randi_range(-1, 1), 1, COLUNAS - 3)

		# a cada ~17 andares, um andar sem paredes — respiro visual e vista aberta
		var andar_aberto: bool = andar > 8 and andar % 17 == 0

		if not andar_aberto:
			var fracao_altura: float = float(andar) / float(andares)
			for col in range(COLUNAS):
				if col == col_vao or col == col_vao + 1:
					continue
				_criar_conteiner(col, andar, p.resistencia_conteiner, fracao_altura)

		# Estação de recarga — o "checkpoint" de energia
		if andar > 2 and andar % int(p.recarga_cada) == 0:
			_criar_recarga(col_vao, andar, p.carga_estacao)

		if andar in andares_de_resgate:
			_criar_resgate(col_vao, andar)
			total_resgates += 1

		if andar > 5 and randf() < float(p.chance_drone):
			_criar_drone(col_vao, andar)

	pos_inicial = Vector3(_x_da_coluna(COLUNAS / 2), ALTURA_CONTEINER, 0)
	pos_helicoptero = Vector3(_x_da_coluna(col_vao), altura_total + 6.0, 0)
	_criar_helicoptero(pos_helicoptero)
	_criar_chao_seguranca()

	nivel_pronto.emit(altura_total)


## Chão de segurança na base — se o jogador cair pelo vão sem voar, pousa
## aqui em vez de cair pro vazio pra sempre. Suaviza a curva de aprendizado
## sem tirar o risco de ficar sem energia lá em cima.
func _criar_chao_seguranca() -> void:
	var corpo := StaticBody3D.new()
	corpo.collision_layer = 2

	var largura_total: float = COLUNAS * LARGURA_CONTEINER + 4.0
	var malha := MeshInstance3D.new()
	var caixa := BoxMesh.new()
	caixa.size = Vector3(largura_total, 2.0, LARGURA_CONTEINER)
	malha.mesh = caixa

	var mat := StandardMaterial3D.new()
	mat.albedo_color = Color(0.16, 0.14, 0.13)
	mat.roughness = 0.9
	malha.material_override = mat
	corpo.add_child(malha)

	var forma := CollisionShape3D.new()
	var box_forma := BoxShape3D.new()
	box_forma.size = caixa.size
	forma.shape = box_forma
	corpo.add_child(forma)

	corpo.position = Vector3(0, -1.0, 0)
	add_child(corpo)


## Espalha os resgates pela torre sem amontoar
func _sortear_andares(quantos: int, minimo: int, maximo: int) -> Array:
	var resultado: Array = []
	var faixa: int = maxi(1, (maximo - minimo) / maxi(1, quantos))
	for i in range(quantos):
		var base: int = minimo + i * faixa
		resultado.append(clampi(base + randi_range(0, faixa - 1), minimo, maximo))
	return resultado


func _limpar() -> void:
	for filho in get_children():
		filho.queue_free()
	total_resgates = 0


func _x_da_coluna(col: int) -> float:
	return (col - COLUNAS / 2.0) * LARGURA_CONTEINER


## Modelos importados vêm com uma malha aninhada dentro de um Node3D
## embrulho — desce a árvore até achar o MeshInstance3D de verdade.
func _achar_mesh_instance(no: Node) -> MeshInstance3D:
	if no is MeshInstance3D:
		return no
	for filho in no.get_children():
		var achado := _achar_mesh_instance(filho)
		if achado:
			return achado
	return null


## Forma de colisão é criada uma vez e reaproveitada por todos os
## contêineres — evita recriar recursos idênticos centenas de vezes por nível.
## A física continua sendo essa caixa simples independente do visual.
func _preparar_recursos_conteiner() -> void:
	if _forma_conteiner:
		return
	_forma_conteiner = BoxShape3D.new()
	_forma_conteiner.size = Vector3(LARGURA_CONTEINER, ALTURA_CONTEINER, LARGURA_CONTEINER)

	for i in range(PALETA_CONTEINER.size()):
		var entrada = PALETA_CONTEINER[i]
		var mat := StandardMaterial3D.new()
		mat.albedo_color = entrada.cor
		mat.roughness = entrada.rugosidade
		mat.metallic = entrada.metalico
		_materiais_conteiner.append(mat)
		_indices_por_zona[entrada.zona].append(i)


## 0 = base (industrial/enferrujado), 1 = corpo, 2 = topo (frio/vívido).
## As faixas de transição (±0.05) misturam um pouco da zona vizinha em vez
## de cortar seco — a torre muda de "clima" aos poucos, não numa linha só.
func _zona_da_altura(fracao: float, hash_local: int) -> int:
	if fracao < 0.30:
		return 0
	if fracao < 0.35 and hash_local % 3 != 0:
		return 0
	if fracao < 0.68:
		return 1
	if fracao < 0.73 and hash_local % 3 != 0:
		return 1
	return 2


func _criar_conteiner(col: int, andar: int, resistencia: int, fracao_altura: float) -> void:
	var corpo := StaticBody3D.new()
	corpo.set_script(preload("res://scripts/Conteiner.gd"))
	corpo.position = Vector3(_x_da_coluna(col), andar * ALTURA_CONTEINER, 0)
	corpo.collision_layer = 2
	corpo.resistencia = resistencia

	var variante: int = (col * 3 + andar * 7) % CENAS_CONTEINER.size()
	var malha := CENAS_CONTEINER[variante].instantiate()
	malha.rotation.y = PI * 0.5  # comprimento natural do modelo (Z) vira a largura visível (X)
	malha.scale = Vector3.ONE * ESCALA_CONTEINER
	malha.position.y = -ALTURA_CONTEINER * 0.5  # pivô do modelo fica na base, não no centro
	corpo.add_child(malha)

	var hash_local: int = col * 5 + andar * 11
	var zona: int = _zona_da_altura(fracao_altura, hash_local)
	var indices: Array = _indices_por_zona[zona]
	var cor_indice: int = indices[hash_local % indices.size()]
	var mesh_instancia := _achar_mesh_instance(malha)
	if mesh_instancia:
		mesh_instancia.material_override = _materiais_conteiner[cor_indice]

	var forma := CollisionShape3D.new()
	forma.shape = _forma_conteiner
	corpo.add_child(forma)

	add_child(corpo)


func _criar_recarga(col: int, andar: int, carga: float) -> void:
	var e := CENA_RECARGA.instantiate()
	e.position = Vector3(_x_da_coluna(col) + LARGURA_CONTEINER * 0.5, andar * ALTURA_CONTEINER, 0)
	e.carga_total = carga
	add_child(e)


func _criar_resgate(col: int, andar: int) -> void:
	var r := CENA_RESGATE.instantiate()
	r.position = Vector3(_x_da_coluna(col) + LARGURA_CONTEINER * 0.5, andar * ALTURA_CONTEINER, 0)
	add_child(r)


func _criar_drone(col: int, andar: int) -> void:
	var d := CENA_DRONE.instantiate()
	d.position = Vector3(_x_da_coluna(col) + LARGURA_CONTEINER * 0.5, andar * ALTURA_CONTEINER, 0)
	d.amplitude = LARGURA_CONTEINER * 0.8
	d.destruido.connect(func(_pos): drone_destruido.emit(PONTOS_DRONE))
	add_child(d)


func _criar_helicoptero(pos: Vector3) -> void:
	var h := CENA_HELICOPTERO.instantiate()
	h.position = pos
	add_child(h)
	helicoptero_atual = h

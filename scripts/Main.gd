extends Node3D
## Orquestrador do jogo: estados, níveis, vidas e pontuação.

enum Estado { MENU, JOGANDO, MORRENDO, NIVEL_COMPLETO, FIM }

@export var vidas_iniciais: int = 5

var estado: Estado = Estado.MENU
var nivel: int = 1
var pontos: int = 0
var vidas: int
var resgates_coletados: int = 0

@onready var gerador: Node3D = $GeradorNivel
@onready var jogador: CharacterBody3D = $Jogador
@onready var camera: Camera3D = $CameraJogo
@onready var hud: Control = $HUD


func _process(_delta: float) -> void:
	if estado != Estado.JOGANDO or gerador.altura_total <= 0.0:
		return

	var fracao_jogador: float = jogador.global_position.y / gerador.altura_total
	var helicoptero_liberado: bool = gerador.helicoptero_atual != null and gerador.helicoptero_atual.liberado
	hud.atualizar_barra_altura(fracao_jogador, 1.0, helicoptero_liberado)


func _ready() -> void:
	vidas = vidas_iniciais
	jogador.morreu.connect(_ao_morrer)
	jogador.resgate_coletado.connect(_ao_coletar_resgate)
	jogador.energia_mudou.connect(hud.atualizar_energia)
	jogador.bombas_mudou.connect(hud.atualizar_bombas)
	hud.jogar_pressionado.connect(iniciar_jogo)
	hud.mostrar_menu()
	AudioManager.tocar_musica()


func iniciar_jogo() -> void:
	nivel = 1
	pontos = 0
	vidas = vidas_iniciais
	_carregar_nivel()


func _carregar_nivel() -> void:
	resgates_coletados = 0
	gerador.gerar(nivel)
	jogador.reviver(gerador.pos_inicial)
	jogador.resgates = 0
	jogador.fase_atual = nivel
	gerador.helicoptero_atual.extracao_feita.connect(_ao_extrair, CONNECT_ONE_SHOT)

	hud.esconder_menu()
	hud.atualizar_nivel(nivel)
	hud.atualizar_vidas(vidas)
	hud.atualizar_pontos(pontos)
	hud.atualizar_resgates(0, gerador.total_resgates)
	estado = Estado.JOGANDO
	_atualizar_marcadores_resgate()

	if nivel == 1:
		_tutorial_fase1()
	else:
		hud.mostrar_aviso("Resgate %d pessoas e suba até o helicóptero" % gerador.total_resgates)


## Sequência de dicas só na primeira fase — a dificuldade suavizada dá tempo
## de sobra pra ler cada uma sem morrer.
func _tutorial_fase1() -> void:
	hud.mostrar_aviso("Segure W (ou ▲) pra voar. Solte pra descer.")
	await get_tree().create_timer(3.2).timeout
	if estado != Estado.JOGANDO:
		return
	hud.mostrar_aviso("A energia drena com o tempo — de olho na barra amarela.")
	await get_tree().create_timer(3.2).timeout
	if estado != Estado.JOGANDO:
		return
	hud.mostrar_aviso("Resgate as %d pessoas (pontos amarelos na barra à direita)." % gerador.total_resgates)
	await get_tree().create_timer(3.2).timeout
	if estado != Estado.JOGANDO:
		return
	hud.mostrar_aviso("Depois suba até o helicóptero pra completar a fase.")


func _atualizar_marcadores_resgate() -> void:
	var fracoes: Array = []
	for r in get_tree().get_nodes_in_group("resgate"):
		fracoes.append(r.global_position.y / gerador.altura_total)
	hud.configurar_barra_altura(fracoes)


func _ao_coletar_resgate(total: int) -> void:
	resgates_coletados = total
	pontos += 500
	hud.atualizar_pontos(pontos)
	hud.atualizar_resgates(total, gerador.total_resgates)
	# queue_free() do resgate ainda não processou — adia pra depois de sair da árvore
	_atualizar_marcadores_resgate.call_deferred()

	if total >= gerador.total_resgates:
		gerador.helicoptero_atual.liberar()
		hud.mostrar_aviso("Todos resgatados! Suba até o helicóptero.")


func _ao_extrair() -> void:
	if estado != Estado.JOGANDO:
		return
	estado = Estado.NIVEL_COMPLETO
	AudioManager.tocar("nivel_completo")

	var bonus: int = int(jogador.energia * 10) + jogador.bombas * 80 + 1000
	pontos += bonus
	hud.atualizar_pontos(pontos)
	hud.mostrar_nivel_completo(bonus, pontos, func():
		nivel += 1
		_carregar_nivel()
	)


func _ao_morrer() -> void:
	if estado != Estado.JOGANDO:
		return
	estado = Estado.MORRENDO
	camera.tremer(1.1)
	hud.mostrar_dano()
	vidas -= 1
	hud.atualizar_vidas(vidas)

	if vidas <= 0:
		estado = Estado.FIM
		AudioManager.tocar("fim_de_jogo")
		hud.mostrar_fim(pontos, nivel, iniciar_jogo)
		return

	await get_tree().create_timer(1.2).timeout
	jogador.reviver(gerador.pos_inicial)
	estado = Estado.JOGANDO

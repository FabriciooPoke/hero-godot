extends Node3D
## Cena usada só pra bater a "foto" cinematográfica do menu (splash) — nunca
## roda durante o jogo em si. Monta uma composição com os mesmos assets do
## jogo (contêineres, personagem, helicóptero) numa pose congelada, pra virar
## uma textura estática de fundo do menu. Tudo montado em código (câmera,
## luzes e ambiente incluídos) pra poder iterar só reexecutando o script.

const LARGURA_CONTEINER := 6.0
const ALTURA_CONTEINER := 3.0

const CENAS_CONTEINER: Array[PackedScene] = [
	preload("res://assets/modelos/shipping-container-a.glb"),
	preload("res://assets/modelos/shipping-container-b.glb"),
	preload("res://assets/modelos/shipping-container-c.glb"),
]
const ESCALA_CONTEINER := LARGURA_CONTEINER / (3.046667 * 0.27)

const PALETA_CONTEINER := [
	{"cor": Color(0.12, 0.36, 0.56), "rugosidade": 0.75, "metalico": 0.2},
	{"cor": Color(0.55, 0.15, 0.13), "rugosidade": 0.75, "metalico": 0.2},
	{"cor": Color(0.17, 0.33, 0.22), "rugosidade": 0.75, "metalico": 0.2},
	{"cor": Color(0.68, 0.52, 0.15), "rugosidade": 0.75, "metalico": 0.2},
	{"cor": Color(0.15, 0.42, 0.43), "rugosidade": 0.75, "metalico": 0.2},
	{"cor": Color(0.52, 0.30, 0.14), "rugosidade": 0.95, "metalico": 0.05},
	{"cor": Color(0.36, 0.23, 0.15), "rugosidade": 0.95, "metalico": 0.0},
]

const CENA_HELICOPTERO := preload("res://scenes/Helicoptero.tscn")
const CENA_JOGADOR := preload("res://scenes/Jogador.tscn")

const POS_JOGADOR := Vector3(-0.5, 9.5, 5.0)


func _ready() -> void:
	DisplayServer.window_set_size(Vector2i(1080, 1920))
	_montar_ambiente()
	_montar_luzes()
	_montar_torre()
	_montar_heli()
	_montar_jogador()
	_montar_camera()
	await get_tree().process_frame
	await get_tree().process_frame
	await get_tree().create_timer(0.15, false).timeout
	_capturar()


## Salva o frame atual como PNG — é assim que a "foto" vira o fundo do menu.
func _capturar() -> void:
	var img := get_viewport().get_texture().get_image()
	var dir := DirAccess.open("res://assets")
	if dir and not dir.dir_exists("imagens"):
		dir.make_dir("imagens")
	var caminho := "res://assets/imagens/tela_abertura.png"
	img.save_png(caminho)
	print("SPLASH SALVA: ", caminho, " ", img.get_size())


func _montar_ambiente() -> void:
	var ceu := ProceduralSkyMaterial.new()
	ceu.sky_top_color = Color(0.04, 0.04, 0.13, 1)
	ceu.sky_horizon_color = Color(0.6, 0.3, 0.16, 1)
	ceu.sky_curve = 0.1
	ceu.ground_bottom_color = Color(0.05, 0.04, 0.06, 1)
	ceu.ground_horizon_color = Color(0.4, 0.17, 0.2, 1)
	ceu.sun_angle_max = 40.0

	var sky := Sky.new()
	sky.sky_material = ceu

	var env := Environment.new()
	env.background_mode = Environment.BG_SKY
	env.sky = sky
	env.ambient_light_source = Environment.AMBIENT_SOURCE_SKY
	env.ambient_light_energy = 1.0
	env.tonemap_mode = Environment.TONE_MAPPER_ACES
	env.ssao_enabled = true
	env.ssao_intensity = 1.5
	env.glow_enabled = true
	env.glow_intensity = 0.9
	env.glow_bloom = 0.25
	env.glow_hdr_threshold = 0.9
	env.fog_enabled = false

	var mundo := WorldEnvironment.new()
	mundo.environment = env
	add_child(mundo)


func _montar_luzes() -> void:
	# Luz-chave: contraluz quente vindo de trás/cima — silhueta dramática
	var chave := DirectionalLight3D.new()
	chave.light_color = Color(1, 0.72, 0.42)
	chave.light_energy = 2.2
	chave.rotation_degrees = Vector3(-25, -150, 0)
	add_child(chave)

	# Preenchimento frio pra dar volume sem apagar o contraluz
	var preenchimento := DirectionalLight3D.new()
	preenchimento.light_color = Color(0.45, 0.55, 0.85)
	preenchimento.light_energy = 0.4
	preenchimento.rotation_degrees = Vector3(-20, -40, 0)
	add_child(preenchimento)


func _montar_torre() -> void:
	var layout := [
		[-3, 0], [-2, 0], [-1, 0], [1, 0], [2, 0], [3, 0],
		[-3, 1], [-2, 1], [-1, 1], [1, 1], [2, 1],
		[-3, 2], [-2, 2], [2, 2], [3, 2],
		[-2, 3], [-1, 3], [1, 3], [2, 3],
		[-2, 4], [2, 4],
		[-1, 5], [1, 5],
	]
	var i := 0
	for par in layout:
		_criar_conteiner(par[0], par[1], i)
		i += 1


func _criar_conteiner(col: int, andar: int, indice: int) -> void:
	var malha := CENAS_CONTEINER[indice % CENAS_CONTEINER.size()].instantiate()
	malha.rotation.y = PI * 0.5
	malha.scale = Vector3.ONE * ESCALA_CONTEINER
	malha.position = Vector3(col * LARGURA_CONTEINER, andar * ALTURA_CONTEINER, 0)
	add_child(malha)

	var entrada = PALETA_CONTEINER[indice % PALETA_CONTEINER.size()]
	var mat := StandardMaterial3D.new()
	mat.albedo_color = entrada.cor
	mat.roughness = entrada.rugosidade
	mat.metallic = entrada.metalico
	var mesh_instancia := _achar_mesh_instance(malha)
	if mesh_instancia:
		mesh_instancia.material_override = mat


func _achar_mesh_instance(no: Node) -> MeshInstance3D:
	if no is MeshInstance3D:
		return no
	for filho in no.get_children():
		var achado := _achar_mesh_instance(filho)
		if achado:
			return achado
	return null


func _montar_heli() -> void:
	var h := CENA_HELICOPTERO.instantiate()
	add_child(h)
	h.position = Vector3(0.0, 21.0, -3.0)
	h.rotation.y = deg_to_rad(15)


func _montar_jogador() -> void:
	var j := CENA_JOGADOR.instantiate()
	add_child(j)
	j.position = POS_JOGADOR
	j.rotation.z = deg_to_rad(-16)
	j.rotation.x = deg_to_rad(-10)
	j.set_physics_process(false)
	j.set_process(false)
	var esquerda: GPUParticles3D = j.get_node("Modelo/Mochila/ChamaEsquerda")
	var direita: GPUParticles3D = j.get_node("Modelo/Mochila/ChamaDireita")
	esquerda.emitting = true
	direita.emitting = true


func _montar_camera() -> void:
	var cam := Camera3D.new()
	add_child(cam)
	cam.fov = 60.0
	cam.position = Vector3(3.5, 3.0, 14.0)
	cam.look_at(POS_JOGADOR + Vector3(0, 3.2, 0), Vector3.UP)
	cam.current = true

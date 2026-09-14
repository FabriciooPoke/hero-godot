extends Area3D
## Dinamite: fica plantada onde foi solta, o pavio pisca e explode destruindo
## contêineres por perto — não cai, senão sai do lugar que você mirou.
## O laser também detona ela na hora, antes do pavio acabar.

@export var tempo_ate_explodir: float = 1.6
@export var raio: float = 8.0
@export var custo_energia_por_bloco: float = 3.0

const EFEITO_EXPLOSAO := preload("res://scenes/efeitos/EfeitoExplosao.tscn")

var _tempo: float = 0.0
var _detonada: bool = false
@onready var pavio: MeshInstance3D = $Pavio

func _process(delta: float) -> void:
	_tempo += delta

	var freq := 6.0 if _tempo < tempo_ate_explodir * 0.6 else 18.0
	pavio.visible = int(_tempo * freq) % 2 == 0

	if _tempo >= tempo_ate_explodir:
		explodir()

func levar_dano_laser() -> void:
	explodir()

func explodir() -> void:
	if _detonada:
		return
	_detonada = true
	AudioManager.tocar("explosao")
	_criar_efeito_visual()

	var camera = get_tree().get_first_node_in_group("camera")
	if camera:
		camera.tremer(0.7)

	var jogador = get_tree().get_first_node_in_group("jogador")

	var espaco := get_world_3d().direct_space_state
	var consulta := PhysicsShapeQueryParameters3D.new()
	var esfera := SphereShape3D.new()
	esfera.radius = raio
	consulta.shape = esfera
	consulta.transform = global_transform
	consulta.collision_mask = 0b1111
	consulta.collide_with_areas = true
	consulta.exclude = [get_rid()]

	for res in espaco.intersect_shape(consulta, 32):
		var alvo = res.collider
		if alvo.has_method("destruir"):
			alvo.destruir()
			# custo só se aplica a contêiner (StaticBody3D) — não a drone (Area3D)
			if alvo is StaticBody3D and jogador and jogador.has_method("gastar_energia"):
				jogador.gastar_energia(custo_energia_por_bloco)
		elif alvo.has_method("levar_dano"):
			alvo.levar_dano()

	queue_free()


func _criar_efeito_visual() -> void:
	var efeito := EFEITO_EXPLOSAO.instantiate()
	get_tree().current_scene.add_child(efeito)
	efeito.global_position = global_position

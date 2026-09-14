extends Area3D
## Drone de patrulha. Mata no toque, morre com laser ou bomba.

signal destruido(pos: Vector3)

const CENA_EXPLOSAO := preload("res://scenes/efeitos/EfeitoExplosao.tscn")

@export var amplitude: float = 5.0
@export var velocidade: float = 1.6

var _t: float = 0.0
var _origem_x: float

func _ready() -> void:
	_origem_x = position.x
	body_entered.connect(_ao_tocar)

func _process(delta: float) -> void:
	_t += delta * velocidade
	position.x = _origem_x + sin(_t) * amplitude
	position.y += sin(_t * 1.7) * delta * 1.2
	$Malha.rotate_y(delta * 9.0)

func _ao_tocar(corpo: Node3D) -> void:
	if corpo.has_method("levar_dano"):
		corpo.levar_dano()

func levar_dano_laser() -> void:
	destruir()

## Destruir também é chamado pela explosão de uma bomba (que já toca seu
## próprio som) — usa um volume mais baixo aqui pra não dobrar o efeito
## quando os dois coincidirem.
func destruir() -> void:
	AudioManager.tocar("explosao", -8.0)
	var efeito := CENA_EXPLOSAO.instantiate()
	get_tree().current_scene.add_child(efeito)
	efeito.global_position = global_position
	destruido.emit(global_position)
	queue_free()

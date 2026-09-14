extends Area3D
## Drone de patrulha. Mata no toque, morre com laser ou bomba.

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

func destruir() -> void:
	queue_free()

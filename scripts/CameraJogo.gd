extends Camera3D
## Câmera 2.5D: segue o jogador com folga e um leve look-ahead.

@export var alvo_path: NodePath
@export var distancia_z: float = 26.0
@export var distancia_extra_queda: float = 6.0  ## quanto afasta a câmera em quedas/subidas rápidas
@export var suavidade: float = 6.0
@export var olhar_a_frente: float = 2.5

var alvo: Node3D
var _shake_forca: float = 0.0
var _shake_t: float = 0.0

func _ready() -> void:
	if alvo_path:
		alvo = get_node(alvo_path)

func _process(delta: float) -> void:
	if alvo == null:
		return

	var vel_vertical: float = absf(alvo.velocity.y)
	var zoom_extra: float = (vel_vertical / alvo.vel_queda_max) * distancia_extra_queda

	var destino := Vector3(
		alvo.global_position.x + alvo.velocity.x * 0.15 * olhar_a_frente,
		alvo.global_position.y + 2.0,
		distancia_z + zoom_extra
	)
	global_position = global_position.lerp(destino, delta * suavidade)

	if _shake_forca > 0.0:
		_shake_t += delta * 40.0
		_shake_forca = maxf(0.0, _shake_forca - delta * 1.6)
		h_offset = sin(_shake_t) * _shake_forca
		v_offset = cos(_shake_t * 1.3) * _shake_forca
	else:
		h_offset = 0.0
		v_offset = 0.0

func tremer(forca: float = 0.6) -> void:
	_shake_forca = forca
	_shake_t = 0.0

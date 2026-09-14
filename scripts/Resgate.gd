extends Area3D
## Pessoa a ser resgatada. Encostar coleta.

signal coletado()

const CENA_EFEITO := preload("res://scenes/efeitos/EfeitoResgate.tscn")

func _ready() -> void:
	body_entered.connect(_ao_tocar)

func _process(delta: float) -> void:
	$Malha.rotate_y(delta * 1.5)
	$Malha.position.y = sin(Time.get_ticks_msec() * 0.003) * 0.25

func _ao_tocar(corpo: Node3D) -> void:
	if corpo.has_method("coletar_resgate"):
		corpo.coletar_resgate()
		coletado.emit()
		AudioManager.tocar("resgate")
		var efeito := CENA_EFEITO.instantiate()
		get_tree().current_scene.add_child(efeito)
		efeito.global_position = global_position
		queue_free()

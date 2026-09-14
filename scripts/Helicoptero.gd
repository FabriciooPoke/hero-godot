extends Area3D
## Objetivo final: só aceita o jogador depois que todos os resgates foram coletados.

signal extracao_feita()

var liberado: bool = false

func _ready() -> void:
	body_entered.connect(_ao_tocar)

func _process(delta: float) -> void:
	$Modelo/Rotor.rotate_y(delta * 30.0)
	$Modelo.position.y = sin(Time.get_ticks_msec() * 0.002) * 0.4

func liberar() -> void:
	liberado = true
	$LuzBusca.light_color = Color(0.4, 1.0, 0.5)

func _ao_tocar(corpo: Node3D) -> void:
	if liberado and corpo.has_method("coletar_resgate"):
		extracao_feita.emit()

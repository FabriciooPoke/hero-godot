extends StaticBody3D
## Contêiner destrutível. O laser danifica; a bomba destrói na hora.
## O visual é um modelo importado (CC0) — a colisão continua sendo uma
## caixa simples independente, então a física do jogo não muda.

@export var resistencia: int = 2

func levar_dano_laser() -> void:
	resistencia -= 1
	if resistencia <= 0:
		destruir()
	else:
		_piscar()

func destruir() -> void:
	# Ponto de extensão: instanciar partículas de destroços aqui
	queue_free()

func _piscar() -> void:
	var malha := _encontrar_mesh(get_child(0))
	if malha == null:
		return
	var original := malha.material_override
	var flash := StandardMaterial3D.new()
	flash.albedo_color = Color(1, 0.8, 0.4)
	flash.emission_enabled = true
	flash.emission = Color(1, 0.8, 0.4)
	flash.emission_energy_multiplier = 2.5
	malha.material_override = flash
	var t := create_tween()
	t.tween_interval(0.08)
	t.tween_callback(func(): malha.material_override = original)


func _encontrar_mesh(no: Node) -> MeshInstance3D:
	if no is MeshInstance3D:
		return no
	for filho in no.get_children():
		var achado := _encontrar_mesh(filho)
		if achado:
			return achado
	return null

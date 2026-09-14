extends StaticBody3D
## Contêiner destrutível. O laser danifica; a bomba destrói na hora.

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
	var malha := get_child(0) as MeshInstance3D
	if malha == null:
		return
	# "flash" é um parâmetro por instância do shader — cada contêiner tem o seu,
	# mesmo compartilhando o mesmo ShaderMaterial com os outros.
	malha.set_instance_shader_parameter("flash", 1.0)
	var t := create_tween()
	t.tween_method(func(v: float): malha.set_instance_shader_parameter("flash", v), 1.0, 0.0, 0.25)

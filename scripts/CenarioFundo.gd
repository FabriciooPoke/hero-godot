extends Node3D
## Cenário de fundo: morros, skyline e mar ao longe — dá profundidade e
## contexto de "fim de tarde litorâneo" à torre sem precisar de asset externo.
## Gerado uma vez, sem física — é decoração pura, em camadas atrás do jogo (Z negativo).

const COR_SILHUETA := Color(0.04, 0.035, 0.07)
const COR_MAR := Color(0.12, 0.22, 0.32)


func _ready() -> void:
	_criar_morros()
	_criar_skyline()
	_criar_mar()


func _criar_morros() -> void:
	var mat := StandardMaterial3D.new()
	mat.albedo_color = COR_SILHUETA
	mat.roughness = 1.0

	for i in range(6):
		var raio: float = randf_range(35.0, 65.0)
		var altura_topo: float = randf_range(15.0, 45.0)

		var m := MeshInstance3D.new()
		var esfera := SphereMesh.new()
		esfera.radius = raio
		esfera.height = raio * 2.0
		m.mesh = esfera
		m.material_override = mat
		m.position = Vector3(randf_range(-90.0, 90.0), altura_topo - raio, -95.0 - randf_range(0.0, 25.0))
		add_child(m)


func _criar_skyline() -> void:
	var mat := StandardMaterial3D.new()
	mat.albedo_color = COR_SILHUETA
	mat.roughness = 1.0

	var x := -85.0
	while x < 85.0:
		var largura: float = randf_range(5.0, 11.0)
		var altura: float = randf_range(10.0, 42.0)

		var m := MeshInstance3D.new()
		var caixa := BoxMesh.new()
		caixa.size = Vector3(largura, altura, largura)
		m.mesh = caixa
		m.material_override = mat
		m.position = Vector3(x + largura * 0.5, altura * 0.5 - 6.0, -55.0)
		add_child(m)

		x += largura + randf_range(1.0, 4.0)


func _criar_mar() -> void:
	var m := MeshInstance3D.new()
	var caixa := BoxMesh.new()
	caixa.size = Vector3(240.0, 26.0, 4.0)
	m.mesh = caixa

	var mat := StandardMaterial3D.new()
	mat.albedo_color = COR_MAR
	mat.roughness = 0.15
	mat.metallic = 0.35
	m.material_override = mat
	m.position = Vector3(0.0, -19.0, -35.0)
	add_child(m)

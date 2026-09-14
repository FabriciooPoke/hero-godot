extends GPUParticles3D
## Efeito de partículas de disparo único — emite e se autodestrói.
## Reaproveitado por qualquer cena de impacto (explosão, faísca do laser).

func _ready() -> void:
	emitting = true
	await get_tree().create_timer(lifetime + 0.15, false).timeout
	queue_free()

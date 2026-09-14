extends Control
## Ícone simples "gire o celular": um retângulo em pé (retrato, sólido) e
## um deitado (paisagem, fantasma) ligados por uma seta curva — sem
## depender de emoji/fonte (não confiável no export Web, ver CLAUDE.md).

func _draw() -> void:
	var centro: Vector2 = size * 0.5
	var largura_tel := size.x * 0.16
	var altura_tel := size.y * 0.34

	# Retrato (alvo) — sólido, à direita
	var pos_retrato := centro + Vector2(size.x * 0.18, 0)
	_desenhar_telefone(pos_retrato, largura_tel, altura_tel, 0.0, Color(1, 1, 1, 0.95))

	# Paisagem (estado atual) — fantasma, à esquerda
	var pos_paisagem := centro + Vector2(-size.x * 0.18, 0)
	_desenhar_telefone(pos_paisagem, largura_tel, altura_tel, PI * 0.5, Color(1, 1, 1, 0.35))

	# Seta curva ligando os dois
	var pontos := 24
	var raio_arco: float = size.x * 0.16
	var pts := PackedVector2Array()
	for i in range(pontos + 1):
		var ang: float = lerp(-PI * 0.65, PI * 0.65, float(i) / pontos)
		pts.append(centro + Vector2(cos(ang), sin(ang)) * raio_arco)
	for i in range(pontos):
		draw_line(pts[i], pts[i + 1], Color(1, 1, 1, 0.6), 4.0, true)
	# Ponta da seta
	var ponta := pts[pontos]
	var direcao := (pts[pontos] - pts[pontos - 1]).normalized()
	var normal := Vector2(-direcao.y, direcao.x)
	draw_colored_polygon(PackedVector2Array([
		ponta + direcao * 14,
		ponta - direcao * 8 + normal * 9,
		ponta - direcao * 8 - normal * 9,
	]), Color(1, 1, 1, 0.6))


func _desenhar_telefone(centro: Vector2, largura: float, altura: float, rotacao: float, cor: Color) -> void:
	draw_set_transform(centro, rotacao, Vector2.ONE)
	var retangulo := Rect2(-largura * 0.5, -altura * 0.5, largura, altura)
	draw_rect(retangulo, cor, false, 5.0)
	draw_set_transform(Vector2.ZERO, 0.0, Vector2.ONE)

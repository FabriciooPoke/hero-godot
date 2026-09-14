extends Control
## Botão touch desenhado em código (sem depender de textura externa) — no
## toque, dispara Input.action_press/release na ação configurada, como se
## fosse um botão físico do teclado. Funciona com toque e com mouse (pra
## testar direto no navegador desktop também).
##
## Visual "marca d'água": quase invisível em repouso (não tampa a visão do
## jogo) e acende só quando pressionado — o padrão usado em jogos mobile
## como Fortnite Mobile e PUBG Mobile. A área de toque continua grande e
## reage no primeiro contato, sem precisar arrastar nem mirar com precisão.

@export var acao: String = ""
## Ícone vetorial (não depende de fonte — evita "tofu" no export Web, que
## não empacota os símbolos ▲◀▶◆ do editor): "cima", "esquerda", "direita",
## "diamante" ou "raio".
@export var icone: String = ""
## "circulo" (usa `raio`, centrado) ou "retangulo" (usa o tamanho todo do
## Control — ótimo pra zonas grandes de toque, tipo mover esquerda/direita).
@export var forma: String = "circulo"
@export var raio: float = 80.0
@export var opacidade_base: float = 0.24
@export var opacidade_pressionado: float = 0.55
## Tinge o botão (ex.: laranja no tiro, pra destacar a ação principal — como
## o botão de fogo colorido em jogos como Free Fire). Branco = neutro.
@export var cor: Color = Color.WHITE

var _pressionado: bool = false
var _dedo: int = -1


func _ready() -> void:
	mouse_filter = Control.MOUSE_FILTER_STOP


func _draw() -> void:
	var alpha_fundo: float = opacidade_pressionado if _pressionado else opacidade_base
	var alpha_icone: float = 0.9 if _pressionado else 0.6

	if forma == "retangulo":
		var retangulo := Rect2(Vector2.ZERO, size)
		draw_rect(retangulo, Color(cor, alpha_fundo))
		draw_rect(retangulo, Color(cor, alpha_fundo * 2.2 + 0.08), false, 3.0)
		if icone != "":
			_desenhar_icone(size * 0.5, minf(size.x, size.y) * 0.3, alpha_icone)
	else:
		var centro: Vector2 = size * 0.5
		# Duas camadas (miolo + brilho externo suave) em vez de um preenchimento
		# chapado — dá um leve efeito de profundidade sem pesar visualmente.
		draw_circle(centro, raio, Color(cor, alpha_fundo * 0.55))
		draw_circle(centro, raio * 0.8, Color(cor, alpha_fundo))
		draw_arc(centro, raio, 0.0, TAU, 48, Color(cor, alpha_fundo * 2.4 + 0.1), 3.0, true)
		if icone != "":
			_desenhar_icone(centro, raio * 0.42, alpha_icone)


func _desenhar_icone(centro: Vector2, t: float, alpha: float) -> void:
	var cor_icone := Color(cor, alpha)
	match icone:
		"cima":
			draw_colored_polygon(PackedVector2Array([centro + Vector2(0, -t), centro + Vector2(-t, t * 0.8), centro + Vector2(t, t * 0.8)]), cor_icone)
		"esquerda":
			draw_colored_polygon(PackedVector2Array([centro + Vector2(-t, 0), centro + Vector2(t * 0.8, -t), centro + Vector2(t * 0.8, t)]), cor_icone)
		"direita":
			draw_colored_polygon(PackedVector2Array([centro + Vector2(t, 0), centro + Vector2(-t * 0.8, -t), centro + Vector2(-t * 0.8, t)]), cor_icone)
		"diamante":
			draw_colored_polygon(PackedVector2Array([centro + Vector2(0, -t), centro + Vector2(t, 0), centro + Vector2(0, t), centro + Vector2(-t, 0)]), cor_icone)
		"raio":
			draw_colored_polygon(PackedVector2Array([
				centro + Vector2(t * 0.15, -t),
				centro + Vector2(-t * 0.55, t * 0.12),
				centro + Vector2(-t * 0.05, t * 0.12),
				centro + Vector2(-t * 0.15, t),
				centro + Vector2(t * 0.55, -t * 0.12),
				centro + Vector2(t * 0.05, -t * 0.12),
			]), cor_icone)


func _gui_input(event: InputEvent) -> void:
	if event is InputEventScreenTouch:
		var toque := event as InputEventScreenTouch
		if toque.pressed and _dedo == -1:
			_dedo = toque.index
			_pressionar()
		elif not toque.pressed and toque.index == _dedo:
			_dedo = -1
			_soltar()
	elif event is InputEventMouseButton and event.button_index == MOUSE_BUTTON_LEFT:
		if event.pressed:
			_pressionar()
		else:
			_soltar()


func _pressionar() -> void:
	if _pressionado:
		return
	_pressionado = true
	Input.action_press(acao)
	queue_redraw()


func _soltar() -> void:
	if not _pressionado:
		return
	_pressionado = false
	Input.action_release(acao)
	queue_redraw()


## Se o dedo/mouse sair da área sem soltar, garante que a ação não fique
## "presa" ligada pro resto da partida.
func _notification(what: int) -> void:
	if what == NOTIFICATION_MOUSE_EXIT and _pressionado and _dedo == -1:
		_soltar()

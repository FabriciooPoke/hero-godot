extends Control
## Botão touch redondo desenhado em código (sem depender de textura externa)
## — no toque, dispara Input.action_press/release na ação configurada, como
## se fosse um botão físico do teclado. Funciona com toque e com mouse (pra
## testar direto no navegador desktop também).

@export var acao: String = ""
## Ícone vetorial (não depende de fonte — evita "tofu" no export Web, que
## não empacota os símbolos ▲◀▶◆ do editor): "cima", "esquerda", "direita"
## ou "diamante". Deixe vazio e use `rotulo` pra texto normal (ex.: "TIRO").
@export var icone: String = ""
@export var rotulo: String = ""
@export var raio: float = 80.0
@export var tamanho_fonte: int = 48

var _pressionado: bool = false
var _dedo: int = -1


func _ready() -> void:
	mouse_filter = Control.MOUSE_FILTER_STOP


func _draw() -> void:
	var centro: Vector2 = size * 0.5
	var cor_fundo: Color = Color(1, 1, 1, 0.38) if _pressionado else Color(1, 1, 1, 0.2)
	draw_circle(centro, raio, cor_fundo)
	draw_arc(centro, raio, 0.0, TAU, 48, Color(1, 1, 1, 0.55), 3.0, true)
	if icone != "":
		_desenhar_icone(centro)
	elif rotulo != "":
		var fonte: Font = ThemeDB.fallback_font
		var tam: Vector2 = fonte.get_string_size(rotulo, HORIZONTAL_ALIGNMENT_CENTER, -1, tamanho_fonte)
		draw_string(fonte, centro - tam * 0.5 + Vector2(0, tam.y * 0.35), rotulo, HORIZONTAL_ALIGNMENT_CENTER, -1, tamanho_fonte, Color(1, 1, 1, 0.92))


func _desenhar_icone(centro: Vector2) -> void:
	var t: float = raio * 0.42
	var cor := Color(1, 1, 1, 0.92)
	match icone:
		"cima":
			draw_colored_polygon(PackedVector2Array([centro + Vector2(0, -t), centro + Vector2(-t, t * 0.8), centro + Vector2(t, t * 0.8)]), cor)
		"esquerda":
			draw_colored_polygon(PackedVector2Array([centro + Vector2(-t, 0), centro + Vector2(t * 0.8, -t), centro + Vector2(t * 0.8, t)]), cor)
		"direita":
			draw_colored_polygon(PackedVector2Array([centro + Vector2(t, 0), centro + Vector2(-t * 0.8, -t), centro + Vector2(-t * 0.8, t)]), cor)
		"diamante":
			draw_colored_polygon(PackedVector2Array([centro + Vector2(0, -t), centro + Vector2(t, 0), centro + Vector2(0, t), centro + Vector2(-t, 0)]), cor)


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

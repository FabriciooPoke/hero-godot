extends Control
## Manche (joystick) virtual "flutuante": aparece onde o dedo tocar dentro
## da área reservada — não precisa mirar num botão fixo — e o manípulo
## segue o dedo dentro de um raio máximo. Mais natural e tolerante a erro
## de mira do que dois botões fixos de esquerda/direita.

@export var raio_base: float = 135.0
@export var raio_manipulo: float = 58.0
@export var zona_morta: float = 0.2  ## fração do raio_base antes de virar "andando"

var _dedo: int = -1
var _com_mouse: bool = false
var _centro_toque: Vector2 = Vector2.ZERO
var _offset_manipulo: Vector2 = Vector2.ZERO
var _ativo: bool = false


func _ready() -> void:
	mouse_filter = Control.MOUSE_FILTER_STOP


func _draw() -> void:
	if not _ativo:
		return
	draw_circle(_centro_toque, raio_base, Color(1, 1, 1, 0.16))
	draw_arc(_centro_toque, raio_base, 0.0, TAU, 48, Color(1, 1, 1, 0.4), 3.0, true)
	draw_circle(_centro_toque + _offset_manipulo, raio_manipulo, Color(1, 1, 1, 0.42))


func _gui_input(event: InputEvent) -> void:
	if event is InputEventScreenTouch:
		var toque := event as InputEventScreenTouch
		if toque.pressed and _dedo == -1:
			_dedo = toque.index
			_iniciar(toque.position)
		elif not toque.pressed and toque.index == _dedo:
			_dedo = -1
			_soltar()
	elif event is InputEventScreenDrag:
		var arraste := event as InputEventScreenDrag
		if arraste.index == _dedo:
			_atualizar(arraste.position)
	elif event is InputEventMouseButton and event.button_index == MOUSE_BUTTON_LEFT:
		if event.pressed:
			_com_mouse = true
			_iniciar(event.position)
		else:
			_com_mouse = false
			_soltar()
	elif event is InputEventMouseMotion and _com_mouse:
		_atualizar(event.position)


func _iniciar(pos: Vector2) -> void:
	# Trava o centro dentro da área visível pra base do manche nunca
	# aparecer cortada nem sair da zona reservada pro polegar.
	_centro_toque = Vector2(
		clampf(pos.x, raio_base, size.x - raio_base),
		clampf(pos.y, raio_base, size.y - raio_base)
	)
	_offset_manipulo = Vector2.ZERO
	_ativo = true
	queue_redraw()


func _atualizar(pos: Vector2) -> void:
	var offset: Vector2 = pos - _centro_toque
	if offset.length() > raio_base:
		offset = offset.normalized() * raio_base
	_offset_manipulo = offset
	queue_redraw()

	var fracao: float = offset.x / raio_base
	if fracao < -zona_morta:
		Input.action_release("mover_direita")
		if not Input.is_action_pressed("mover_esquerda"):
			Input.action_press("mover_esquerda")
	elif fracao > zona_morta:
		Input.action_release("mover_esquerda")
		if not Input.is_action_pressed("mover_direita"):
			Input.action_press("mover_direita")
	else:
		Input.action_release("mover_esquerda")
		Input.action_release("mover_direita")


func _soltar() -> void:
	_ativo = false
	Input.action_release("mover_esquerda")
	Input.action_release("mover_direita")
	queue_redraw()


## Se o dedo sumir sem soltar (interrupção do SO, troca de app etc.), não
## deixa o personagem andando sozinho pro resto da partida.
func _notification(what: int) -> void:
	if what == NOTIFICATION_MOUSE_EXIT and _com_mouse:
		_com_mouse = false
		_soltar()

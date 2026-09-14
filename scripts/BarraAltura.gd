extends Control
## Barra vertical fixa mostrando onde o jogador está na torre e onde ficam
## os objetivos restantes (resgates + helicóptero). Fica sempre visível,
## diferente do aviso de texto que pisca e some.

@onready var jogador_marcador: Label = $Jogador
@onready var helicoptero_marcador: Label = $Helicoptero

var _marcadores_resgate: Array[Label] = []


func atualizar_jogador(fracao: float) -> void:
	_posicionar(jogador_marcador, fracao)


func atualizar_helicoptero(fracao: float, liberado: bool) -> void:
	_posicionar(helicoptero_marcador, fracao)
	helicoptero_marcador.modulate = Color(0.4, 1.0, 0.55) if liberado else Color(0.55, 0.55, 0.6)


func atualizar_resgates(fracoes: Array) -> void:
	for m in _marcadores_resgate:
		m.queue_free()
	_marcadores_resgate.clear()

	for f in fracoes:
		var m := Label.new()
		m.text = "*"
		m.add_theme_font_size_override("font_size", 20)
		m.modulate = Color(1, 0.78, 0.25)
		m.mouse_filter = MOUSE_FILTER_IGNORE
		add_child(m)
		m.size = Vector2(24, 24)
		_marcadores_resgate.append(m)
		_posicionar(m, f)


func _posicionar(marcador: Label, fracao: float) -> void:
	var altura_disponivel: float = size.y - marcador.size.y
	marcador.position.x = (size.x - marcador.size.x) * 0.5
	marcador.position.y = altura_disponivel * (1.0 - clampf(fracao, 0.0, 1.0))

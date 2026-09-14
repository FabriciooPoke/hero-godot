extends Control
## HUD: pontuação, energia, bombas e os controles touch pro celular.

signal jogar_pressionado()
signal pausar_alternado()
signal sair_para_menu_pressionado()

@onready var lbl_pontos: Label = $Topo/Pontos
@onready var lbl_nivel: Label = $Topo/Nivel
@onready var lbl_vidas: Label = $Topo/Vidas
@onready var lbl_resgates: Label = $Topo/Resgates
@onready var barra_energia: ProgressBar = $Rodape/BarraEnergia
@onready var lbl_bombas: Label = $Rodape/Bombas
@onready var painel_menu: Control = $PainelMenu
@onready var painel_fim: Control = $PainelFim
@onready var lbl_aviso: Label = $Aviso
@onready var barra_altura: Control = $BarraAltura
@onready var flash_dano: ColorRect = $FlashDano
@onready var painel_pausa: Control = $PainelPausa
@onready var slider_musica: HSlider = $PainelMenu/Volumes/SliderMusica
@onready var slider_efeitos: HSlider = $PainelMenu/Volumes/SliderEfeitos
@onready var slider_musica_pausa: HSlider = $PainelPausa/Volumes/SliderMusica
@onready var slider_efeitos_pausa: HSlider = $PainelPausa/Volumes/SliderEfeitos

## Godot "canvas_items + expand" mantém 1 unidade de canvas = 1px físico só
## no eixo que está "espremido" contra o tamanho base (1080x1920) — o outro
## eixo "sobra". Em retrato quem aperta é a largura (unidade ≈ 1080); em
## paisagem quem aperta é a altura (unidade ≈ 1920, só que física bem menor).
## Resultado: um raio fixo em unidades de canvas fica ~45% menor na tela em
## paisagem. Por isso os botões são posicionados/dimensionados nas cenas
## pensando numa "unidade curta" de referência (1080, o valor base do
## projeto) e reescalados aqui sempre que o viewport muda de proporção.
const UNIDADE_BASE := 1080.0
var _controles_base: Array = []


func _ready() -> void:
	painel_fim.visible = false
	painel_pausa.visible = false
	lbl_aviso.visible = false
	$PainelMenu/BotaoJogar.pressed.connect(func(): jogar_pressionado.emit())
	$PainelPausa/BotaoContinuar.pressed.connect(func(): pausar_alternado.emit())
	$PainelPausa/BotaoMenu.pressed.connect(func(): sair_para_menu_pressionado.emit())
	# Só mostra os botões touch em quem realmente tem tela sensível ao toque —
	# no desktop (mesmo via navegador) eles só atrapalhariam a visão.
	var em_touch: bool = OS.has_feature("mobile") or DisplayServer.is_touchscreen_available()
	$Controles.visible = em_touch
	if em_touch:
		_preparar_controles_responsivos()

	for slider in [slider_musica, slider_musica_pausa]:
		slider.value = AudioManager.volume_musica()
		slider.value_changed.connect(AudioManager.definir_volume_musica)
	for slider in [slider_efeitos, slider_efeitos_pausa]:
		slider.value = AudioManager.volume_efeitos()
		slider.value_changed.connect(AudioManager.definir_volume_efeitos)


## Guarda o tamanho/posição "desenhados" na cena (pensados pra retrato) de
## cada botão, e reescala tudo pela unidade curta atual sempre que o
## viewport mudar — inclusive na primeira vez, pro caso de já abrir
## paisagem.
func _preparar_controles_responsivos() -> void:
	for no in [$Controles/Esquerda, $Controles/Direita, $Controles/Voar, $Controles/Atirar, $Controles/Bomba]:
		_controles_base.append({
			"no": no,
			"raio": no.raio,
			"l": no.offset_left, "t": no.offset_top, "r": no.offset_right, "b": no.offset_bottom,
		})
	get_viewport().size_changed.connect(_redimensionar_controles)
	_redimensionar_controles()


func _redimensionar_controles() -> void:
	var vp: Vector2 = get_viewport().get_visible_rect().size
	var unidade: float = minf(vp.x, vp.y)
	var fator: float = unidade / UNIDADE_BASE
	for c in _controles_base:
		var no: Control = c.no
		no.raio = c.raio * fator
		no.offset_left = c.l * fator
		no.offset_top = c.t * fator
		no.offset_right = c.r * fator
		no.offset_bottom = c.b * fator
		no.queue_redraw()


func _unhandled_input(event: InputEvent) -> void:
	if event.is_action_pressed("pausar"):
		pausar_alternado.emit()
		get_viewport().set_input_as_handled()


func mostrar_menu() -> void:
	painel_menu.visible = true
	painel_fim.visible = false

func esconder_menu() -> void:
	painel_menu.visible = false
	painel_fim.visible = false


func atualizar_pontos(v: int) -> void:
	lbl_pontos.text = "PONTOS\n%06d" % v

func atualizar_nivel(v: int) -> void:
	lbl_nivel.text = "TORRE %02d" % v

func atualizar_vidas(v: int) -> void:
	lbl_vidas.text = "VIDAS  " + "^".repeat(maxi(0, v))

func atualizar_resgates(feitos: int, total: int) -> void:
	lbl_resgates.text = "RESGATES %d/%d" % [feitos, total]

func atualizar_energia(v: float) -> void:
	barra_energia.value = v
	var estilo := barra_energia.get_theme_stylebox("fill") as StyleBoxFlat
	if estilo:
		estilo.bg_color = Color(0.9, 0.2, 0.15) if v < 25.0 else Color(0.95, 0.72, 0.15)

func atualizar_bombas(v: int) -> void:
	lbl_bombas.text = "BOMBAS " + "*".repeat(maxi(0, v))


func configurar_barra_altura(fracoes_resgates: Array) -> void:
	barra_altura.atualizar_resgates(fracoes_resgates)


func atualizar_barra_altura(fracao_jogador: float, fracao_helicoptero: float, liberado: bool) -> void:
	barra_altura.atualizar_jogador(fracao_jogador)
	barra_altura.atualizar_helicoptero(fracao_helicoptero, liberado)


func mostrar_pausa(mostrar: bool) -> void:
	painel_pausa.visible = mostrar


func mostrar_dano() -> void:
	flash_dano.color.a = 0.45
	var t := create_tween()
	t.tween_property(flash_dano, "color:a", 0.0, 0.4)


func mostrar_aviso(texto: String) -> void:
	lbl_aviso.text = texto
	lbl_aviso.visible = true
	var t := create_tween()
	t.tween_interval(2.5)
	t.tween_callback(func(): lbl_aviso.visible = false)


func mostrar_nivel_completo(bonus: int, total: int, ao_continuar: Callable) -> void:
	painel_fim.visible = true
	$PainelFim/Titulo.text = "EXTRAÇÃO COMPLETA"
	$PainelFim/Detalhe.text = "Bônus: +%d\nTotal: %d" % [bonus, total]
	$PainelFim/BotaoAcao.text = "PRÓXIMA TORRE"
	_religar_botao($PainelFim/BotaoAcao, func():
		painel_fim.visible = false
		ao_continuar.call()
	)


func mostrar_fim(pontos: int, nivel: int, ao_reiniciar: Callable) -> void:
	painel_fim.visible = true
	$PainelFim/Titulo.text = "FIM DE JOGO"
	$PainelFim/Detalhe.text = "Pontos: %d\nTorre: %d" % [pontos, nivel]
	$PainelFim/BotaoAcao.text = "JOGAR DE NOVO"
	_religar_botao($PainelFim/BotaoAcao, func():
		painel_fim.visible = false
		ao_reiniciar.call()
	)


func _religar_botao(botao: Button, acao: Callable) -> void:
	for c in botao.pressed.get_connections():
		botao.pressed.disconnect(c.callable)
	botao.pressed.connect(acao)

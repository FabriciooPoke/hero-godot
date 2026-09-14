extends CharacterBody3D
## Roderick Hero — voo com mochila-jetpack em 2.5D.
## O movimento acontece no plano XY; Z fica travado em 0.

signal energia_mudou(valor: float)
signal bombas_mudou(qtd: int)
signal morreu()
signal resgate_coletado(total: int)

# --- Ajuste de "feel" do voo. Mexa aqui pra calibrar a jogabilidade. ---
@export var impulso_jetpack: float = 34.0      ## força pra cima ao segurar o botão
@export var gravidade: float = 18.0            ## peso da queda
@export var vel_subida_max: float = 11.0
@export var vel_queda_max: float = 14.0
@export var vel_lateral_ar: float = 9.0
@export var vel_lateral_chao: float = 11.0
@export var aceleracao_lateral: float = 42.0   ## quanto mais alto, mais responsivo
@export var atrito_lateral: float = 34.0
@export var amortecimento_teto: float = 30.0   ## suaviza a chegada nos tetos de velocidade vertical
@export var impulso_inicial: float = 0.12      ## fração do impulso aplicada instantaneamente ao apertar (evita sensação de atraso)

# --- Recursos ---
@export var energia_max: float = 100.0
@export var dreno_parado: float = 1.2          ## energia/segundo só existindo (na dificuldade máxima)
@export var dreno_voando: float = 5.5          ## energia/segundo com jetpack ligado (na dificuldade máxima)
@export var custo_laser: float = 2.0
@export var bombas_iniciais: int = 6

## Fase atual — controla a curva de dificuldade do dreno (ver _fator_dreno()).
## Setado pelo Main ao carregar cada nível.
var fase_atual: int = 1

@export var alcance_laser: float = 14.0
@export var raio_explosao: float = 8.0

var energia: float
var bombas: int
var olhando_para: int = 1                       ## 1 = direita, -1 = esquerda
var invulneravel: float = 0.0
var vivo: bool = true
var resgates: int = 0

@onready var helice: Node3D = $Modelo/Mochila/Helice
@onready var chama_esq: GPUParticles3D = $Modelo/Mochila/ChamaEsquerda
@onready var chama_dir: GPUParticles3D = $Modelo/Mochila/ChamaDireita
@onready var raio_laser: RayCast3D = $RaioLaser
@onready var visual_laser: MeshInstance3D = $VisualLaser
@onready var modelo: Node3D = $Modelo

@onready var _partes_corpo: Array[MeshInstance3D] = [
	$Modelo/Corpo, $Modelo/Capacete, $Modelo/BracoEsquerdo, $Modelo/BracoDireito,
	$Modelo/PernaEsquerda, $Modelo/PernaDireita, $Modelo/Mochila/CorpoMochila,
]
var _materiais_originais: Array[Material] = []

const CENA_BOMBA := preload("res://scenes/Bomba.tscn")
const MAT_FLASH_DANO := preload("res://resources/materiais/flash_dano.tres")


func _ready() -> void:
	energia = energia_max
	bombas = bombas_iniciais
	energia_mudou.emit(energia)
	bombas_mudou.emit(bombas)
	for parte in _partes_corpo:
		_materiais_originais.append(parte.material_override)
	visual_laser.visible = false
	raio_laser.target_position = Vector3(alcance_laser, 0, 0)


func _physics_process(delta: float) -> void:
	if not vivo:
		return

	if invulneravel > 0.0:
		invulneravel -= delta
		modelo.visible = int(invulneravel * 12) % 2 == 0
	else:
		modelo.visible = true

	_processar_voo(delta)
	_processar_lateral(delta)
	_processar_energia(delta)
	_processar_acoes()

	move_and_slide()

	# trava o eixo Z — jogo é 2.5D
	global_position.z = 0.0

	helice.rotate_y(delta * (26.0 if Input.is_action_pressed("propulsar") else 9.0))


func _processar_voo(delta: float) -> void:
	var propulsando := Input.is_action_pressed("propulsar") and energia > 0.0

	if Input.is_action_just_pressed("propulsar") and energia > 0.0:
		velocity.y = maxf(velocity.y, impulso_jetpack * impulso_inicial)

	if propulsando:
		velocity.y += impulso_jetpack * delta
	velocity.y -= gravidade * delta

	# capa suave: perto do teto/piso de velocidade, ela se aproxima em vez de travar seco
	if velocity.y > vel_subida_max:
		velocity.y = move_toward(velocity.y, vel_subida_max, amortecimento_teto * delta)
	elif velocity.y < -vel_queda_max:
		velocity.y = move_toward(velocity.y, -vel_queda_max, amortecimento_teto * delta)

	chama_esq.emitting = propulsando
	chama_dir.emitting = propulsando

	# inclina o corpo conforme sobe/desce — dá peso visual ao voo
	var alvo_pitch := clampf(-velocity.y * 0.04, -0.5, 0.5)
	modelo.rotation.z = lerpf(modelo.rotation.z, alvo_pitch * olhando_para, delta * 8.0)


func _processar_lateral(delta: float) -> void:
	var dir := Input.get_axis("mover_esquerda", "mover_direita")
	var vel_alvo := (vel_lateral_chao if is_on_floor() else vel_lateral_ar) * dir

	if absf(dir) > 0.1:
		olhando_para = signi(int(dir))
		velocity.x = move_toward(velocity.x, vel_alvo, aceleracao_lateral * delta)
		modelo.rotation.y = lerp_angle(modelo.rotation.y, 0.0 if olhando_para > 0 else PI, delta * 12.0)
	else:
		velocity.x = move_toward(velocity.x, 0.0, atrito_lateral * delta)


## Fases iniciais drenam bem mais devagar — sobe até o valor "cheio" na fase 5+.
## Suaviza a curva de aprendizado sem tirar a tensão das fases avançadas.
func _fator_dreno() -> float:
	return clampf(0.4 + (fase_atual - 1) * 0.15, 0.4, 1.0)


func _processar_energia(delta: float) -> void:
	var dreno: float = (dreno_voando if Input.is_action_pressed("propulsar") else dreno_parado) * _fator_dreno()
	energia = maxf(0.0, energia - dreno * delta)
	energia_mudou.emit(energia)
	if energia <= 0.0:
		levar_dano()


func _processar_acoes() -> void:
	if Input.is_action_just_pressed("atirar") and energia > custo_laser:
		disparar_laser()
	if Input.is_action_just_pressed("bomba") and bombas > 0:
		soltar_bomba()


func disparar_laser() -> void:
	energia = maxf(0.0, energia - custo_laser)
	energia_mudou.emit(energia)
	AudioManager.tocar("laser")

	raio_laser.target_position = Vector3(alcance_laser * olhando_para, 0, 0)
	raio_laser.force_raycast_update()

	_piscar_laser()

	if raio_laser.is_colliding():
		var alvo := raio_laser.get_collider()
		if alvo.has_method("levar_dano_laser"):
			alvo.levar_dano_laser()


func _piscar_laser() -> void:
	visual_laser.visible = true
	visual_laser.scale.x = alcance_laser
	visual_laser.position.x = (alcance_laser * 0.5) * olhando_para
	var t := create_tween()
	t.tween_interval(0.07)
	t.tween_callback(func(): visual_laser.visible = false)


func soltar_bomba() -> void:
	bombas -= 1
	bombas_mudou.emit(bombas)
	var b := CENA_BOMBA.instantiate()
	get_tree().current_scene.add_child(b)
	b.global_position = global_position
	b.raio = raio_explosao


func coletar_resgate() -> void:
	resgates += 1
	resgate_coletado.emit(resgates)


func recarregar_energia(valor: float) -> void:
	energia = minf(energia_max, energia + valor)
	energia_mudou.emit(energia)


func levar_dano() -> void:
	if invulneravel > 0.0 or not vivo:
		return
	vivo = false
	chama_esq.emitting = false
	chama_dir.emitting = false
	_flash_impacto()
	AudioManager.tocar("dano")
	morreu.emit()


## Pisca o corpo todo de vermelho por um instante — feedback claro de que
## o golpe conectou e uma vida foi perdida.
func _flash_impacto() -> void:
	for parte in _partes_corpo:
		parte.material_override = MAT_FLASH_DANO
	var t := create_tween()
	t.tween_interval(0.15)
	t.tween_callback(func():
		for i in _partes_corpo.size():
			_partes_corpo[i].material_override = _materiais_originais[i]
	)


func reviver(pos: Vector3) -> void:
	global_position = pos
	velocity = Vector3.ZERO
	energia = energia_max
	energia_mudou.emit(energia)
	vivo = true
	invulneravel = 2.0

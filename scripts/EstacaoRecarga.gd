extends Area3D
## Estação de recarga: devolve energia enquanto o jogador encosta.
## Cada estação tem carga limitada — depois de esgotar, fica inativa por um
## tempo antes de poder ser usada de novo.

signal esgotada()
signal reativada()

@export var carga_total: float = 70.0      ## quanta energia esta estação tem pra dar
@export var taxa: float = 45.0             ## energia por segundo transferida
@export var tempo_reativacao: float = 10.0 ## segundos até voltar a carregar depois de esgotar

const COR_ATIVA := Color(0.3, 1, 0.6, 1)
const COR_ESGOTADA := Color(0.3, 0.3, 0.35, 1)

var carga_restante: float
var _jogador_dentro: Node3D = null
var _tempo_esgotada: float = 0.0

@onready var malha: MeshInstance3D = $Malha
@onready var luz: OmniLight3D = $Luz


func _ready() -> void:
	carga_restante = carga_total
	body_entered.connect(_ao_entrar)
	body_exited.connect(_ao_sair)


func _process(delta: float) -> void:
	if carga_restante <= 0.0:
		_tempo_esgotada += delta
		if _tempo_esgotada >= tempo_reativacao:
			_reativar()

	# pulsa enquanto tem carga
	var proporcao: float = carga_restante / carga_total
	luz.light_energy = 1.0 + sin(Time.get_ticks_msec() * 0.006) * 0.6 * proporcao

	var mat := malha.material_override as StandardMaterial3D
	if mat:
		mat.emission_energy_multiplier = 0.4 + 3.0 * proporcao

	if _jogador_dentro == null or carga_restante <= 0.0:
		return

	var transferir: float = minf(taxa * delta, carga_restante)
	_jogador_dentro.recarregar_energia(transferir)
	carga_restante -= transferir

	if carga_restante <= 0.0:
		_tempo_esgotada = 0.0
		luz.light_color = COR_ESGOTADA
		esgotada.emit()


func _reativar() -> void:
	carga_restante = carga_total
	luz.light_color = COR_ATIVA
	reativada.emit()


func _ao_entrar(corpo: Node3D) -> void:
	if corpo.has_method("recarregar_energia"):
		_jogador_dentro = corpo


func _ao_sair(corpo: Node3D) -> void:
	if corpo == _jogador_dentro:
		_jogador_dentro = null

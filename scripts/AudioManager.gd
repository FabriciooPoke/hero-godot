extends Node
## Autoload central de efeitos sonoros. Acesso desacoplado de qualquer
## script do jogo: AudioManager.tocar("laser").
##
## Usa um pool fixo de AudioStreamPlayer (round-robin) em vez de instanciar
## um player novo a cada som — evita alocação por frame e permite sons
## sobrepostos (ex: laser disparado repetidamente) sem cortar um ao outro.

const SONS := {
	"laser": preload("res://assets/audio/sfx/laser.wav"),
	"explosao": preload("res://assets/audio/sfx/explosao.wav"),
	"resgate": preload("res://assets/audio/sfx/resgate.wav"),
	"dano": preload("res://assets/audio/sfx/dano.wav"),
	"nivel_completo": preload("res://assets/audio/sfx/nivel_completo.wav"),
	"fim_de_jogo": preload("res://assets/audio/sfx/fim_de_jogo.wav"),
}

const TAMANHO_POOL := 8

var _pool: Array[AudioStreamPlayer] = []
var _proximo: int = 0


func _ready() -> void:
	for i in range(TAMANHO_POOL):
		var p := AudioStreamPlayer.new()
		add_child(p)
		_pool.append(p)


func tocar(nome: String, volume_db: float = 0.0) -> void:
	if not SONS.has(nome):
		push_warning("AudioManager: som '%s' não existe" % nome)
		return
	var player := _pool[_proximo]
	_proximo = (_proximo + 1) % _pool.size()
	player.stream = SONS[nome]
	player.volume_db = volume_db
	player.play()

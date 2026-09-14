extends Node
## Autoload central de áudio. Acesso desacoplado de qualquer script do jogo:
## AudioManager.tocar("laser"), AudioManager.tocar_musica().
##
## Efeitos usam um pool fixo de AudioStreamPlayer (round-robin) em vez de
## instanciar um player novo a cada som — evita alocação por frame e permite
## sons sobrepostos (ex: laser disparado repetidamente) sem cortar um ao outro.
##
## Música e efeitos ficam em buses separados ("Musica" e "Efeitos") pra dar
## controle de volume independente.

const SONS := {
	"laser": preload("res://assets/audio/sfx/laser.wav"),
	"explosao": preload("res://assets/audio/sfx/explosao.wav"),
	"resgate": preload("res://assets/audio/sfx/resgate.wav"),
	"dano": preload("res://assets/audio/sfx/dano.wav"),
	"nivel_completo": preload("res://assets/audio/sfx/nivel_completo.wav"),
	"fim_de_jogo": preload("res://assets/audio/sfx/fim_de_jogo.wav"),
	"recarregando": preload("res://assets/audio/sfx/recarregando.wav"),
}

var _musica_fundo: AudioStreamWAV = preload("res://assets/audio/musica/aventura.wav")

const BUS_MUSICA := "Musica"
const BUS_EFEITOS := "Efeitos"
const TAMANHO_POOL := 8

var _pool: Array[AudioStreamPlayer] = []
var _proximo: int = 0
var _player_musica: AudioStreamPlayer


func _ready() -> void:
	var buses_novos := _garantir_buses()
	if buses_novos:
		definir_volume_musica(0.6)
		definir_volume_efeitos(0.8)

	for i in range(TAMANHO_POOL):
		var p := AudioStreamPlayer.new()
		p.bus = BUS_EFEITOS
		add_child(p)
		_pool.append(p)

	_musica_fundo.loop_mode = AudioStreamWAV.LOOP_FORWARD
	_player_musica = AudioStreamPlayer.new()
	_player_musica.bus = BUS_MUSICA
	_player_musica.stream = _musica_fundo
	add_child(_player_musica)


## Cria os buses "Musica" e "Efeitos" se ainda não existirem — assim o jogo
## funciona mesmo sem configuração manual no editor. Retorna true se algum
## bus precisou ser criado (usado pra aplicar volumes padrão só na 1ª vez).
func _garantir_buses() -> bool:
	var criou_algum := false
	for nome in [BUS_MUSICA, BUS_EFEITOS]:
		if AudioServer.get_bus_index(nome) == -1:
			var idx := AudioServer.bus_count
			AudioServer.add_bus(idx)
			AudioServer.set_bus_name(idx, nome)
			AudioServer.set_bus_send(idx, "Master")
			criou_algum = true
	return criou_algum


func tocar(nome: String, volume_db: float = 0.0) -> void:
	if not SONS.has(nome):
		push_warning("AudioManager: som '%s' não existe" % nome)
		return
	var player := _pool[_proximo]
	_proximo = (_proximo + 1) % _pool.size()
	player.stream = SONS[nome]
	player.volume_db = volume_db
	player.play()


func tocar_musica() -> void:
	if not _player_musica.playing:
		_player_musica.play()


func parar_musica() -> void:
	_player_musica.stop()


## volume: 0.0 (mudo) a 1.0 (cheio)
func definir_volume_musica(volume: float) -> void:
	AudioServer.set_bus_volume_db(AudioServer.get_bus_index(BUS_MUSICA), linear_to_db(clampf(volume, 0.0001, 1.0)))


func definir_volume_efeitos(volume: float) -> void:
	AudioServer.set_bus_volume_db(AudioServer.get_bus_index(BUS_EFEITOS), linear_to_db(clampf(volume, 0.0001, 1.0)))


func volume_musica() -> float:
	return db_to_linear(AudioServer.get_bus_volume_db(AudioServer.get_bus_index(BUS_MUSICA)))


func volume_efeitos() -> float:
	return db_to_linear(AudioServer.get_bus_volume_db(AudioServer.get_bus_index(BUS_EFEITOS)))

# H.E.R.O. Reloaded — projeto Godot

Base jogável do jogo, escrita em GDScript. A mecânica está completa e funcionando;
o visual está em **blocos coloridos primitivos** (BoxMesh/CapsuleMesh), que é exatamente
o lugar onde a arte 3D entra depois.

---

## 1. Rodar o projeto

1. Baixe o Godot 4.7 em https://godotengine.org/download (versão estável atual, gratuito, sem licença)
2. Abra o Godot → **Import** → aponte para o `project.godot` desta pasta
3. Aperte **F5** (ou o botão ▶ no canto superior direito)

Controles no teclado: `A`/`D` ou setas para mover, `W` ou `↑` para voar
(soltar a tecla = cair), `Espaço` para o laser, `B` para a bomba. No celular
aparecem os botões touch.

---

## 1.5. Leia o DESIGN.md primeiro

Antes de mexer em qualquer número, leia o `DESIGN.md`. Ele explica a intenção do
jogo, por que a energia funciona do jeito que funciona, e como a dificuldade
cresce de fase em fase. Se você mudar valores sem entender isso, é fácil quebrar
a tensão que faz o jogo funcionar.

---

## 2. O que já funciona

- Voo com jetpack (impulso vs. gravidade), inclinação do corpo conforme sobe/desce
- Torres de contêineres geradas proceduralmente, com vão que serpenteia
- Contêineres **destrutíveis** — laser danifica, bomba destrói uma área
- Drones patrulhando, matam no toque, morrem com laser/bomba
- Resgates espalhados pela torre; depois de coletar todos, o helicóptero libera a extração
- Barra de energia que drena (mais rápido voando) — zerou, perdeu uma vida
- **Estações de recarga** com carga limitada, espalhadas pela torre
- **Perfis de dificuldade por fase** (fases 1-4 desenhadas à mão, 5+ escalam)
- Objetivo anunciado na tela ao começar cada fase
- Sistema de vidas, pontuação, bônus no fim do nível, torre seguinte mais alta
- Câmera 2.5D com suavização e *look-ahead*
- HUD completo + controles touch pra celular

---

## 3. Calibrar a jogabilidade (sem programar)

Abra `scenes/Jogador.tscn`, clique no nó raiz e mexa no Inspector. Todos esses
valores estão expostos como `@export`:

| Propriedade | O que faz |
|---|---|
| `impulso_jetpack` | Força pra cima. Maior = sobe mais rápido |
| `gravidade` | Peso da queda. Menor = flutua mais |
| `vel_subida_max` / `vel_queda_max` | Teto de velocidade vertical |
| `vel_lateral_ar` / `vel_lateral_chao` | Velocidade horizontal |
| `aceleracao_lateral` | Quanto maior, mais responsivo ao toque |
| `dreno_voando` | Consumo de energia com o jetpack ligado |

Em `scenes/Main.tscn` → nó `GeradorNivel`: `andares_base` e `andares_por_nivel`
controlam o tamanho das torres.

---

## 4. Trocar os blocos pela arte 3D

Esta é a parte que dá o visual da referência. O código **não precisa mudar** —
ele só procura os nós pelo nome.

### Personagem
1. Coloque o modelo (`.glb` ou `.fbx`) em `assets/modelos/`
2. Abra `scenes/Jogador.tscn`
3. Apague os nós `Corpo` e `Capacete` dentro de `Modelo`
4. Arraste o `.glb` pra dentro de `Modelo`
5. Mantenha os nomes: o script procura por `Modelo/Mochila/Helice`, `ChamaEsquerda`,
   `ChamaDireita`. Reposicione esses nós no modelo novo (a hélice no topo da mochila,
   as chamas nos bocais).

### Contêineres
Em `scripts/GeradorNivel.gd`, função `_criar_conteiner()`. Troque o bloco que cria
`BoxMesh` + `StandardMaterial3D` por:

```gdscript
const MODELO_CONTEINER := preload("res://assets/modelos/conteiner.glb")
# ...
var visual := MODELO_CONTEINER.instantiate()
corpo.add_child(visual)
```

Mantenha o `CollisionShape3D` com o mesmo tamanho — é ele que define a física.

### Cenário de fundo (skyline, mar, morros)
Adicione um `WorldEnvironment` com skybox HDRI e alguns `MeshInstance3D` distantes
em `Main.tscn`. Como a câmera é fixa em Z, dá pra usar planos com textura sem
quebrar a ilusão.

### Iluminação
Já existem duas luzes direcionais em `Main.tscn` (uma quente de frente, uma fria de
trás) e glow ligado no `default_env.tres`. É o esqueleto da luz de fim de tarde
da referência — ajuste `light_color` e `light_energy` conforme a arte chegar.

---

## 5. Onde conseguir a arte 3D

- **Assets prontos**: Sketchfab, Kenney.nl (gratuito), Quaternius, Unity/Unreal Marketplace
- **Sob encomenda**: Workana, 99Freelas, Fiverr, ArtStation — modelador 3D para jogos
- **Gerar**: ferramentas de texto-para-3D (Meshy, Luma) servem pra rascunho;
  qualidade final ainda pede modelador

O que você precisa encomendar, em ordem de impacto visual:
personagem com jetpack (animado) → contêiner (2-3 variações) → helicóptero →
drone → skybox de fim de tarde.

---

## 6. Exportar para iOS, Android e web

No Godot: **Project → Export → Add**.

| Plataforma | O que precisa |
|---|---|
| **Web (HTML5)** | Nada além do Godot. Exporta e sobe em qualquer hospedagem |
| **Android** | Android SDK + chave de assinatura. Gera `.apk`/`.aab`. Conta Google Play: US$ 25 (única) |
| **iOS** | Exporta o projeto Xcode, mas o build final **exige um Mac com Xcode**. Conta Apple: US$ 99/ano |

A primeira exportação pede os *export templates* — o Godot baixa sozinho quando
você clica.

---

## 7. Estrutura

```
project.godot          configuração + mapeamento de controles
scenes/
  Main.tscn            cena principal (jogo todo)
  Jogador.tscn         personagem + jetpack + laser
  HUD.tscn             interface e botões touch
  Resgate.tscn         pessoa a resgatar
  Drone.tscn           inimigo
  Bomba.tscn           dinamite
  Helicoptero.tscn     objetivo final
scripts/               um .gd por cena, em português
resources/             ambiente (céu, fog, glow)
assets/                vazio — a arte 3D entra aqui
```

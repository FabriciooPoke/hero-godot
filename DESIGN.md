# DESIGN — H.E.R.O. Reloaded

Este documento existe pra responder uma pergunta: **por que alguém jogaria isso?**
Se uma decisão de código contradiz o que está aqui, o documento ganha.

---

## A intenção em uma frase

> Você é um socorrista com combustível contado subindo uma torre que está caindo.
> Cada segundo no ar custa. Cada pessoa salva custa mais.

---

## O loop central

A energia drena **sempre**, e drena ~4x mais rápido com o jetpack ligado.
Isso transforma cada momento numa decisão:

```
      subir agora?                    economizar?
           │                               │
     gasta energia                   perde tempo
     ganha altura                    energia drena igual
           │                               │
           └──────── a torre não espera ───┘
```

Não existe jogar "seguro" — ficar parado também drena energia, só que mais
devagar. Essa é a tensão que segura o jogador **nas fases avançadas**.

**Curva de dificuldade suavizada (2026):** o dreno começa em 40% do valor
máximo na Fase 1 e sobe ~15%/fase até chegar em 100% na Fase 5 — ver
`Jogador._fator_dreno()`. A ideia central continua a mesma, mas o jogador
novo aprende o jogo sem ser punido por existir. Além disso, a base da torre
tem um **chão de segurança**: cair pelo vão sem voar pousa ali, nunca é
queda infinita. A Fase 1 também tem uma sequência curta de dicas guiadas
(`Main._tutorial_fase1()`). Isso é uma mudança deliberada de rumo, pedida
explicitamente — prioriza "fácil e divertido" sobre a dureza arcade original.

**O que o jogador quer:** chegar ao helicóptero no topo.
**O que o impede:** a energia acaba antes.
**O que ele decide:** a rota. Qual caminho gasta menos? Vale desviar pra pegar
aquele resgate longe? Gasto laser pra abrir atalho ou contorno voando?

---

## Por que tem estação de recarga

Sem recarga, a energia era um cronômetro burro: acabava e você morria, sem nada
a fazer. Com recarga, ela vira um **recurso que se administra**.

Regras que fazem a estação funcionar:

1. **Carga limitada.** Cada estação dá uma quantidade fixa (60–80) e esgota.
   Não dá pra estacionar nela e ficar imortal.
2. **Reativa depois de 10s seca.** Esgotou, fica escura por 10 segundos e volta
   a carregar — dá pra usar de novo se você voltar, mas não é instantâneo.
3. **Fica no caminho, não no destino.** Estão no vão de subida — você passa por elas
   naturalmente, a decisão é *parar ou seguir*, não *achar*.

O cálculo que o jogador faz o tempo todo: *"tenho 40 de energia e a próxima
estação está 6 andares acima — dá?"*

---

## Como a dificuldade cresce

O botão principal **não** é "mais inimigos". É a **distância entre estações**.
Quanto mais longe uma da outra, mais o jogador precisa planejar em vez de improvisar.

| Fase | Andares | Estação a cada | Carga | Resgates | Drones | O que ensina |
|---|---|---|---|---|---|---|
| 1 | 20 | 4 andares | 80 | 3 | raro | Voar e resgatar. Energia sobra de propósito |
| 2 | 26 | 5 andares | 70 | 4 | pouco | Aparecem inimigos. Desviar custa energia |
| 3 | 32 | 6 andares | 65 | 5 | médio | Contêiner com 2 de resistência: laser fica caro |
| 4 | 38 | 8 andares | 60 | 6 | bastante | Estações escassas. Obriga a planejar a rota |
| 5+ | +6/fase | até 10 | mín. 45 | até 9 | até 40% | Escala contínua com piso de justiça |

A fase 1 é generosa **de propósito**. Ninguém aprende um jogo morrendo.

---

## O critério de vitória de cada fase

Duas condições, nessa ordem:

1. **Coletar todos os resgates** da torre (o HUD mostra `RESGATES 2/5`)
2. **Chegar ao helicóptero** no topo

O helicóptero só aceita você depois do item 1 — o holofote dele muda pra verde
quando libera. Isso impede o atalho óbvio de ignorar as pessoas e subir reto,
que mataria a intenção do jogo.

**Pontuação:**
- 500 por resgate
- 1000 por completar a fase
- Energia restante × 10 (prêmio por eficiência)
- Bombas não usadas × 80 (prêmio por não depender de força bruta)

O bônus de energia é importante: ele recompensa exatamente o comportamento que
o jogo quer ensinar — subir gastando pouco.

---

## Decisões que valem discutir depois

Coisas que deixei como estão, mas que podem mudar o jogo pra melhor:

- **Resgatado como peso.** Carregar uma pessoa poderia aumentar o dreno de
  energia, forçando a levá-la logo ao helicóptero. Vira "viagens" em vez de
  "coleta". Mudaria bastante o ritmo.
- **A torre desabando.** Um limite de tempo real por fase (a torre cede de baixo
  pra cima) daria urgência visual, não só numérica.
- **Bomba como recurso de rota**, não arma. Abrir atalho na horizontal pra
  economizar voo vertical é mais interessante do que matar drone.
- **Placar online.** O jogo é curto e repetível, formato que vive de recorde.

---

## O que NÃO fazer

- Não colocar recarga automática por tempo — mata a tensão inteira
- Não deixar o jogador subir ignorando os resgates — mata a intenção
- Não resolver dificuldade só jogando mais inimigos na tela — vira ruído
- Não tirar o bônus de energia restante — é o que ensina a jogar bem

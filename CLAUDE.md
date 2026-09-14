# H.E.R.O. Reloaded

Jogo 2.5D de resgate com jetpack (estilo H.E.R.O. clássico), feito em Godot 4.7.2 / GDScript. Torres verticais de contêineres, resgates, bombas, laser, estações de recarga, helicóptero de extração.

- **Godot binário:** `/Applications/Godot.app/Contents/MacOS/Godot`
- **Link publicado (sempre a versão mais recente):** https://hero-godot.vercel.app — NUNCA usar um link `hero-godot-<hash>-fabricio-eufrazio.vercel.app`, esses ficam presos num deploy específico e antigo.
- **Deploy:** GitHub (`FabriciooPoke/hero-godot`, branch `main`) → Vercel via integração Git, automático a cada push. `git push` funciona sem digitar nada (token já em cache no Keychain do macOS) — nunca peça senha/token ao usuário nem digite um por ele.
- Fluxo padrão de mudanças: skill `melhorar-jogo` (`.claude/skills/melhorar-jogo/SKILL.md`).

## Pegadinhas do motor já encontradas (não repita o diagnóstico)

- **`default_env.tres` pode ser corrompido pelo `--export-debug`/`--headless --editor --import`.** Sempre compare o `md5` antes/depois de exportar.
- **GDScript `:=` falha ao acessar propriedades via variável tipada como classe-base/ancestral.** Use `: Tipo` explícito nesses casos.
- **`set_instance_shader_parameter()` derruba a renderização silenciosamente** se o parâmetro não estiver declarado como `instance uniform` no shader atualmente atribuído.
- **`RayCast3D` não detecta `Area3D`** sem `collide_with_areas = true` (default é `false`).
- **`TouchScreenButton` sem `texture_normal` nem `shape` é invisível e não recebe toque** — por isso os controles touch usam um `Control` desenhado em código (`BotaoTouch.gd`), não `TouchScreenButton`.
- **GLB importado pode ter escala interna escondida no nó** (achado: 0.27), separada da AABB da malha — sempre confira a cadeia de transform completa, não só `get_aabb()`.
- **`ProceduralSkyMaterial` renderiza branco no Web export do Safari/WebKit** (pipeline de radiância HDR não suportado). `default_env.tres` usa `background_mode = Color` (cor sólida) por causa disso — não trocar de volta pra Sky sem testar no Safari de verdade.
- **`Environment.fog_enabled` tinge o fundo inteiro pra cor do fog**, mesmo com densidade baixa, porque o fundo é renderizado a distância "infinita" (fórmula `1-exp(-densidade*distância)` tende a 1). Fog está desativado por causa disso.
- **Export Web não empacota os glyphs Unicode `▲ ◀ ▶ ◆ ●`** (viram "tofu"/quadrado vazio) — o app nativo usa fallback de fonte do sistema, o Web não. Usar ASCII (`^ < > *`) ou ícones vetoriais desenhados (`draw_colored_polygon`), nunca esses símbolos em texto.
- **`create_timer()` por padrão ignora `get_tree().paused`** (`process_always = true`). Passe `false` explícito quando o timer precisa respeitar a pausa.
- **Emissão/luz forte demais com tonemap ACES dessatura pra branco** em vez de manter a cor — é assim que a estação de recarga e os resgates ficaram "brancos" e viraram bug reportado. Mantenha `emission_energy_multiplier` moderado (~1.0–1.6) em objetos coloridos importantes.

## Preferências do usuário

- Nunca inserir credenciais, senhas ou tokens por ele — mesmo com autorização explícita. Login/geração de token é sempre feito pelo próprio usuário no navegador/terminal dele.
- Testar de verdade (rodar o jogo, screenshot, emulação de toque) antes de dizer que algo está pronto — não presumir que uma mudança visual funcionou sem olhar.

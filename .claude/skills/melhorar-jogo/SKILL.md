---
name: melhorar-jogo
description: Fluxo padrão para implementar melhorias, correções ou novas funcionalidades no H.E.R.O. Reloaded (Godot) — edita, valida, exporta pra Web, testa e publica. Use sempre que o pedido for "melhora o jogo", "corrige X", "adiciona Y" ou qualquer mudança no gameplay/visual/áudio do projeto Godot.
disable-model-invocation: false
---

# Melhorias no H.E.R.O. Reloaded

Fluxo de trabalho padrão deste projeto (Godot 4.7.2, `/Users/fabricioeufrazio/Desktop/hero-godot`). Siga estes passos pra qualquer mudança de código/cena/recurso:

## 1. Implementar
Edite os `.gd`/`.tscn`/`.tres` necessários. Consulte o `CLAUDE.md` na raiz do projeto pra pegadinhas conhecidas do motor antes de mexer em Environment, partículas, input touch ou shaders.

## 2. Checar erros (headless, rápido)
```bash
cd /Users/fabricioeufrazio/Desktop/hero-godot
/Applications/Godot.app/Contents/MacOS/Godot --headless --quit-after 2 2>&1 | tail -30
```
Não deve haver `ERROR`/`Parse Error` além dos avisos inofensivos já conhecidos (`icon.svg`, RID leak ao sair).

## 3. Exportar pra Web
```bash
md5 resources/default_env.tres   # antes
/Applications/Godot.app/Contents/MacOS/Godot --headless --export-debug "Web" build/web/index.html 2>&1 | tail -10
md5 resources/default_env.tres   # depois — TEM que ser igual
```
Se o md5 mudar, o `--export-debug` corrompeu o arquivo (já aconteceu antes — perdeu o `ProceduralSkyMaterial`). Restaure via `git diff`/`git checkout` e investigue antes de prosseguir.

## 4. Testar de verdade
Sirva o build localmente (evite `/tmp` puro — use o diretório de scratchpad) e abra no navegador (`mcp__Claude_Browser__*`):
```bash
mkdir -p <scratchpad>/webtest && cp -r build/web/* <scratchpad>/webtest/
python3 -c "
import http.server, socketserver, functools
Handler = functools.partial(http.server.SimpleHTTPRequestHandler, directory='<scratchpad>/webtest')
class S(socketserver.TCPServer): allow_reuse_address = True
with S(('0.0.0.0', 8793), Handler) as h: h.serve_forever()
" &
```
- Mudança de UI/controle touch → `resize_window(preset:"mobile")` antes de navegar, testar toque de verdade.
- Sempre checar `read_console_messages(onlyErrors:true)` — só o erro de `icon.svg` é esperado.
- Mudança visual → tirar screenshot e olhar de fato, não assumir.

## 5. Commitar e publicar
```bash
git add -A
git commit -m "$(cat <<'EOF'
<resumo objetivo do que mudou e por quê>

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
EOF
)"
GIT_TERMINAL_PROMPT=0 git push
```
O `git push` funciona sozinho (token já em cache no Keychain do macOS). Se um dia voltar a pedir senha, avise o usuário — não digite nada no prompt.

## 6. Confirmar no ar
O link estável (sempre aponta pro deploy mais recente) é **https://hero-godot.vercel.app** — NÃO usar links `hero-godot-<hash>-fabricio-eufrazio.vercel.app`, esses ficam presos num deploy específico. Deploy geralmente fica pronto em segundos após o push.

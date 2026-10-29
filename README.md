# BC Drums

Plugin do BC Studio: **LR da bateria crua entra, LR pronta sai.**
Três controles: **Estilo**, **Transient Design** e **Mix**.

## Formatos
- **Plugin**: AU (Logic), VST3 e CLAP. Funciona no BC Sound Lab, Logic, Reaper etc.
- **App separado** (`BC Drums.app`): arraste o WAV, escolha o estilo e exporte.
- **Linha de comando** (`bcdrums`): `bcdrums processar entrada.wav saida.wav --estilo Worship`.

## Packs de timbres (como atualizar)
Os packs ficam em `~/Music/BC Drums/Packs/`. Cada pasta é um **estilo**:

```
Packs/
  Worship/
    pack.toml        (opcional: nome e ajustes da cadeia do estilo)
    bumbo/      01.wav 02.wav … (do mais fraco para o mais forte)
    caixa/      01.wav …
    tom_agudo/  01.wav …
    tom_grave/  01.wav …
```

1. Copie/troque os WAVs (qualquer taxa, 16/24/32 bits, mono ou estéreo).
2. No plugin, clique em **Recarregar** (ou **Abrir pasta** para ir direto nela).
3. Para criar um estilo novo, duplique uma pasta (ex.: `Worship` → `Rock`) e troque os samples.

Nomes de pasta aceitos: `bumbo`/`kick`, `caixa`/`snare`, `tom_agudo`/`rack`, `tom_grave`/`floor`/`surdo`.
Peça sem pasta = o original segue tratado, sem reforço de sample.

## Como funciona
1. O detector acha cada batida na LR (bumbo, caixa, tom agudo, tom grave) e mede a força.
2. O áudio é atrasado ~40 ms (o host compensa sozinho).
3. Cada batida dispara o sample do pack na camada certa de dinâmica, alinhado com o original.
4. Mixagem completa da LR em 4 faixas (grave/bumbo, corpo, ataque, pratos) com Transient Design, compressão e ganho por faixa, abertura dos pratos → reforço opcional de samples → cola + paralela → reverb → saturação → limitador.
5. Mix entre o original e o resultado.

## Desenvolvimento
```
cargo test --workspace --exclude xtask
cargo xtask bundle bcd-plugin --release
cargo run -p bcd-cli -- analisar teste.wav
```
O instalador Mac é gerado pelo workflow `bc-drums-mac.yml` e publicado no branch `bc-drums-instalador`.

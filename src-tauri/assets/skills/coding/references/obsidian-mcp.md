# Registro do MCP do second brain

O servidor é o [@bitbonsai/mcpvault](https://github.com/bitbonsai/mcpvault), registrado **em
escopo de usuário** com o nome `second-brain`, apontando para a pasta global que `brain.py where`
mostra. Ele é opcional: sem MCP, `brain.py` e a leitura local fazem a consulta.

Confira o CLI do cliente, Node.js e npx. Passe o caminho absoluto do second brain como um único
argumento: ele tem espaço (`second brain`), então mantenha as aspas. O separador `--` pertence ao
CLI, não ao comando salvo do servidor.

Inspecione o registro antes de adicionar. Reutilize um registro equivalente. Se `second-brain` já
aponta para outra pasta ou serviço, explique o conflito e peça a escolha. Não reaproveite nem
substitua um servidor `obsidian` existente, que pode servir a outro cofre.

**Claude Code**:

```sh
claude mcp get second-brain
claude mcp add --scope user second-brain -- npx -y @bitbonsai/mcpvault "<caminho do second brain>"
claude mcp get second-brain
```

**Codex** (o registro pelo CLI já vale para o usuário):

```sh
codex mcp get second-brain
codex mcp add second-brain -- npx -y @bitbonsai/mcpvault "<caminho do second brain>"
codex mcp get second-brain
```

**Windows (PowerShell)**: o PowerShell descarta um `--` solto antes de repassá-lo ao CLI, e o
`claude` então falha com `unknown option '-y'`. Escreva o separador entre aspas simples, `'--'`.
Obtenha o caminho com `py scripts/brain.py where` e passe-o entre aspas. Se o cliente não iniciar
o shim `npx`, use o `node` com o caminho absoluto verificado do `npx-cli.js` da instalação do npm.
Este formato funciona:

```powershell
claude mcp add --scope user second-brain '--' "C:\Program Files\nodejs\node.exe" "C:\Program Files\nodejs\node_modules\npm\bin\npx-cli.js" -y "@bitbonsai/mcpvault" "<caminho do second brain>"
codex mcp add second-brain '--' "C:\Program Files\nodejs\node.exe" "C:\Program Files\nodejs\node_modules\npm\bin\npx-cli.js" -y "@bitbonsai/mcpvault" "<caminho do second brain>"
```

Confira o caminho do `npx-cli.js` antes (`(Get-Command node).Source` mostra a pasta do Node).
Evite montar uma linha `cmd.exe` com caminhos do usuário. No WSL, use o Node, o CLI e o caminho do
próprio WSL.

**OpenCode**: na configuração global (`~/.config/opencode/opencode.json`), mescle só a entrada
`mcp.second-brain`, preservando o resto: `{"type":"local","command":["npx","-y",
"@bitbonsai/mcpvault","<caminho absoluto>"],"enabled":true}`. Confira com `opencode mcp list`.

Depois de registrar, confira as ferramentas anunciadas e faça uma leitura limitada da raiz.
Distinga registro gravado de conexão testada. Se o cliente só carrega servidores na próxima sessão,
diga para reabrir. Mostre ao usuário qualquer falha ao baixar o pacote ou iniciar o processo.

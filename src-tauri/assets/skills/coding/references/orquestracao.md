# Pilar 2 — Orquestração inteligente

Recriado a partir da Alethe Orchestrate v24 (MIT, Kauã Miguel), com o nome Coding Opus. Termine o
trabalho com o menor contexto possível no lead e os workers mais baratos que dão conta, ou com a
combinação mais forte quando o usuário pedir. Planeje, delegue unidades limitadas e integre
resultados sustentados por evidência.

Caminhos como `scripts/...` e `references/...` são relativos à raiz da skill. Os comandos abaixo
são subcomandos de `scripts/orchestrate.py` (no Windows, `py scripts/orchestrate.py <comando>`).

- **Playbook** = as fases e os papéis.
- **Perfil** = quais modelos cada tier usa (`base`, `savings`, `aggressive`).
- **Objetivo** = quanto delegar com um modo ligado (`passive` por padrão; `save`, `balanced`,
  `max`, `freestyle` quando pedidos).
- **Estratégia do runner** = como uma onda roda (`economy`, `balanced`, `quality`, `research`,
  `parallel`, `rescue`).

> **No Coding Agent** (ferramentas `coding_agent_*` disponíveis), quem sobe os workers é o app:
> siga [modo-app.md](modo-app.md) e use deste arquivo só playbooks, cota, briefs e disciplina de tokens.

## Estado e nomes

| Item | Onde |
| --- | --- |
| política, playbooks, preços, cota | `%APPDATA%\CodingOpus\coding-opus-*.json/yaml` (Windows) ou `~/.config/coding-opus/` |
| orquestrações | `orchestrations/` dentro dessa pasta |
| worktrees dos workers | `.coding-opus/worktrees/` no repositório |
| variáveis | `CODING_OPUS_POLICY`, `CODING_OPUS_PLAYBOOKS`, `CODING_OPUS_HOME`, `CODING_OPUS_PASSIVE`, `CODING_OPUS_WORKER`, `CODING_OPUS_SESSION` |

Na primeira execução, se a pasta nova não existe e há uma pasta da Alethe (`%APPDATA%\Alethe` ou
`~/.config/alethe`), o runner **copia** o estado para a pasta nova e renomeia `alethe-*` para
`coding-opus-*`. A pasta antiga fica intacta. O arquivo `MIGRATED_FROM.txt` registra a origem.

## Escolher o backend

Respeite um pedido explícito pela skill local ou sem MCP: ele sempre vence. Senão, rode
`policy show` e use o `backend` dele. `skill` seleciona este runner. `mcp` seleciona as
ferramentas `alethe_*` do app Alethe quando existirem. `auto` prefere essas ferramentas quando
disponíveis.

Python é opcional. Se `python --version`, `python3 --version` e `py --version` falharem ou
ficarem abaixo de 3.9, ou se o usuário pedir, siga [manual.md](manual.md) para detecção, opções,
workers em segundo plano e relatório. As outras seções continuam valendo pelos equivalentes manuais.

O caminho da skill não precisa de MCP. Ele roda workers concorrentes limitados, com isolamento
opcional em worktree e sessões retomáveis. Workers CLI não aparecem no quadro do app Alethe.

## Primeira execução e configuração

Rode `doctor` quando o ambiente puder estar despreparado ou o usuário perguntar o que está
configurado. Ele confere Python, provedores, a política salva e a cópia dos playbooks, e resume as
leituras de cota com dicas práticas.

Quando `doctor` não mostrar política configurada, ou o usuário pedir para configurar a
orquestração, siga [onboarding.md](onboarding.md):
- levante os CLIs, modelos e rotas;
- monte os perfis `base`, `savings` e `aggressive`;
- peça aprovação antes de salvá-los na cópia dos playbooks. Salve só preferências confirmadas.

Um proxy como o 9router carrega tráfego de modelo; ele não é um worker.

A configuração nunca bloqueia o trabalho. No meio de uma tarefa sem nada configurado, rode
`doctor`, use os CLIs logados com os playbooks embutidos, diga isso em uma linha e continue.
Ofereça o onboarding depois da etapa atual; não salve nada sem o ok.

Traduza pedidos soltos de configuração em ações:

| O usuário diz | Fazer |
| --- | --- |
| "poupe o provedor X" como preferência | confirme e rode `policy set --conserve-providers X` |
| "use modelos baratos nas coisas pequenas" | confirme provedor e modelo e edite `tiers` na cópia dos playbooks |
| "no máximo N workers / T minutos / C dólares" como preferência | confirme e rode `policy set --max-calls N --max-wall-seconds T --max-reported-cost-usd C` |
| qualquer um acima "só nesta execução" | use o `budget` do plano ou `--strategy`; nunca toque na política |
| "no máximo N chamadas / C dólares / T tokens nesta tarefa" | `budget` com esses limites antes da primeira onda |
| "onde paramos?", ou depois de uma compactação | `status` antes de tudo |
| "volte ao padrão" | confirme e rode `policy reset` |
| "mude os playbooks" ou os papéis por fase | `playbook --init` e edite a cópia que ele mostra; o editor exporta o mesmo arquivo |
| "use savings" ou "modo agressivo" como preferência | confirme e defina `profile` na cópia dos playbooks |
| "o que fez?", "quanto custou?" | `report`, sem confirmação |

## Comandos curtos

`/coding <args>` no Claude Code, ou as mesmas palavras depois de
"coding opus" em outro cliente, viram uma ação. Comandos de uma execução sem pasta usam a
orquestração atual: a que `new` iniciou, `use` escolheu ou o último `run` gravou.

| Palavras | Fazer |
| --- | --- |
| uma tarefa, em palavras simples | orquestre-a como o resto deste arquivo descreve |
| nada | `status`; sem orquestração atual, liste estes comandos em poucas linhas |
| `on`, ou uma instrução permanente ("daqui em diante...") | fique ligado, como abaixo |
| `always`, "sempre ligado", "em todo projeto" | `mode on --everywhere` |
| `off` | `mode off`; com o padrão ligado, desliga só neste projeto |
| `off everywhere` | `mode off --everywhere` |
| `passive` | `mode on --goal passive` |
| `spend [--transcript PATH] [--session ID] [--json]` | `spend` com essas opções; mostre a saída como está |
| `save`, `balanced`, `max`, `freestyle` | `mode on --goal <ele>` |
| `careful` | `mode on --care careful` |
| `report [pasta]` | `report`, e mostre a saída como está, sem resumo seu |
| `report html`, "mostre o relatório" | `report --format html`, que grava `report.html` na pasta e abre; sem Python, preencha `assets/report.html` como em [manual.md](manual.md) |
| `status` | `status`, e mostre a saída como está |
| `usage` | `usage`; `--refresh-claude` quando a leitura do Claude estiver ausente ou velha, `--refresh-agy` idem para o Antigravity; mostre janelas, resets e idade |
| `models` | `models`; mostre preços, ranks e idade dos preços |
| `editor`, "mostre os playbooks" | `editor`, que grava os playbooks como página de fluxo e abre; sem Python, preencha `assets/editor.html` como em [manual.md](manual.md) |
| `list` | `list`; `use <pasta>` torna uma delas a atual de novo |
| `stop` | `stop` e depois `status` quando as ondas terminarem |
| `clean [--older-than DIAS] [--yes]` | prévia das pastas concluídas com mais de 14 dias; apague só com `--yes` |
| `budget <limite>` | `budget --max-calls N`, `--max-reported-cost-usd C` ou `--max-reported-tokens T` |
| `skip <fase>`, `switch <playbook>`, `freeform`, `add <fase>` | mude o rumo como abaixo |
| `note <texto>` | `note "<texto>"` |
| `mark <worker> used\|partial\|discarded` | `mark <worker> <marca>` |
| `setup` | `doctor` e onboarding quando não houver política salva |
| `help` | liste estes comandos em poucas linhas |

## Modo passivo (padrão)

Trabalhe normalmente, sem playbook, plano ou ondas, a menos que o usuário peça. O modo passivo
vigia o gasto desta sessão do lead e a cota dos provedores. Ele delega só quando um brief curto e
uma unidade limitada tornam um worker mais barato que o lead.

Sem perguntar, use apenas workers por assinatura com cota fresca sobrando. O `billing` em
`coding-opus-prices.json` tem `subscription` como padrão. Um provedor cobrado por API, ou uma cota
desconhecida ou velha, exige o ok do usuário antes, inclusive para os subagentes do próprio agente.

Unidades elegíveis:
- reconhecimento ou busca somente leitura em muitos arquivos;
- edições mecânicas com verificação automática;
- escrever ou rodar testes;
- revisão independente de um diff pronto.

Passos pequenos, trabalho ambíguo ou de design, edições acopladas e a integração ficam com o lead.
O fluxo com playbook abaixo vale quando pedido explicitamente.

**Preferência pelo Codex.** Prefira o Codex enquanto as janelas de 5h e da semana dele forem
conhecidas, frescas e abaixo de `quota.criticalThreshold`: a cota de assinatura que sobra no reset
se perde.
- Respeite o `billing` em `coding-opus-prices.json` e `passive.prefer`. Compare preços de API e
  pergunte antes de usar esse provedor, mesmo que as janelas de limite tenham folga.
- Rode cada unidade limitada do Codex como uma onda `economy` de um worker, com `new`,
  `run --detach` e `wait`, fixando modelo e esforço.
- Quando o Codex estiver baixo, ou a unidade precisar do contexto desta conversa, use os
  subagentes mais baratos do próprio agente. No Claude Code, é a ferramenta Agent com
  `model: haiku` ou `sonnet`, e um esforço fixo quando houver.
- Cota desconhecida não é prova de folga: pergunte antes de rotear com leituras desconhecidas ou
  velhas, sem atualizar o Claude a partir de um hook. Subagentes mais baratos seguem as mesmas
  regras de cobrança e cota.

**Como avisar.** Diga em uma linha ao delegar, sem pedir confirmação: "Passando a varredura de
testes ao Codex (5h em 4%) para poupar o Opus."
- Pergunte antes de passar de `passive.askAboveWorkers` (padrão 2) workers em paralelo, de usar um
  provedor baixo, de algo que custe mais do que fazer você mesmo, ou de passar de `quota.stopAt`.
- Quando o gasto subir em trabalho rotineiro, sugira `/model sonnet` uma vez; o modelo do lead é
  escolha do usuário.

**Hook e gasto.** O hook dá um brief curto no primeiro prompt e depois fica quieto até uma checagem
de tempo, gasto ou cota vencer, inclusive em turnos longos de ferramenta.
- `spend` lê o JSONL do Claude Code localmente, separa o uso do lead do delegado e mostra uma
  estimativa a preço de lista. É uma régua de assinatura, não uma fatura.
- `/coding off` desliga neste projeto; `/coding off everywhere`, em todos. `/coding passive` liga
  explicitamente aqui.
- `CODING_OPUS_PASSIVE=0` desliga só o padrão implícito; modos explícitos continuam valendo.
- Workers nunca recebem o contexto do hook. Sem o hook, rode `mode show` no início da sessão e
  depois de uma compactação, e `spend`/`usage` antes de trabalho substancial.

Veja [playbooks.md](playbooks.md) para os ajustes opcionais de `passive`.

## Ficar ligado

1. Use proativamente sempre que um pedido se dividir em unidades independentes e limitadas, mesmo
   sem ninguém falar em orquestração.
   - Antes de uma orquestração não pedida com várias ondas ou vários workers, mostre poucas linhas
     do `dry-run` (playbook, workers, provedores/modelos, estimativa, cota) e espere o ok. Isso vale
     também sob `balanced`, `save` e `max`: ligar um objetivo não é aprovação.
   - Ficam isentas as delegações passivas de um worker a provedores por assinatura com folga
     fresca, dentro de `passive.askAboveWorkers`; as orquestrações pedidas explicitamente; e as
     instruções permanentes que mandam seguir.
   - Trabalho pequeno ou acoplado fica com você.
2. `mode on --everywhere` vale em todos os projetos por 30 dias desde a última vez que foi ligado;
   chamar de novo renova, e `mode show` mostra `expiresAt`. Depois do prazo, volta o padrão passivo,
   que não expira. `mode off` desliga aqui; `mode off --everywhere` desliga o padrão passivo em todo
   lugar, assim como `CODING_OPUS_PASSIVE=0`.
3. `mode on` mantém ligado no projeto atual (a raiz do repositório, ou a pasta) até `mode off`, ou
   depois de 12 horas sem prompt. Um modo de projeto vence o padrão.

Os dois modos aceitam as mesmas opções:

- `--goal save` gasta o mínimo; `max` tira o máximo dos agentes disponíveis; `balanced` (padrão)
  delega só quando o playbook diz que compensa.
- `--goal freestyle` dispensa playbooks, planos e ondas: você trabalha normalmente e só vigia o
  custo.
  - Confira a cota antes de passos longos ou pesados.
  - Quando houver rota mais barata ou melhor, diga em uma linha, no máximo uma vez por resposta e
    não de novo se recusada: "Esta renomeação é mecânica; vou passar a um worker Sonnet para sair
    barato."
  - Pegue uma rota mais barata que mantenha a qualidade necessária e diga isso nessa linha.
    Pergunte antes de algo que custe mais, inicie vários workers ou passe da linha de parada.
  - Seu modelo é escolha do usuário: sugira `/model <nome>`.
  - Não confunda com `freeform`, que dispensa o playbook de uma orquestração em andamento.
- `--care careful` lê antes de escrever, isola todo writer, revisa todo diff antes de integrar e
  pergunta antes de algo difícil de desfazer.
- `--rule "<palavras do usuário>"` guarda uma instrução permanente, até 8; `--clear-rules` as apaga.

Ligue quando o usuário disser `/coding on`, der uma instrução permanente ou pedir orquestração
"daqui em diante". Mapeie a instrução para um objetivo ou nível de cuidado quando couber e guarde
as palavras do usuário como regra. Confirme em uma linha o que ficou ligado e volte ao que ele
fazia.

O hook do Claude Code lembra a cada prompt nos modos explícitos não passivos. Sem ele, rode
`mode show` no início da sessão e depois de uma compactação. Delegue só onde o custo extra compensa
sob o objetivo; trabalho pequeno fica com você.

## Entrar no meio da tarefa

Entre em qualquer ponto, inclusive dentro do fluxo de outra skill. Não recomece:

1. Use o que já existe como plano: a conversa, uma spec, uma lista de tarefas, um design doc, o
   diff até agora. Aponte os workers para esses arquivos em vez de copiá-los.
2. Mantenha as etapas e checagens do outro fluxo: tarefas pendentes viram unidades e as checagens
   viram gates. Reescreva os arquivos dele só quando uma etapa dele mandar.
3. Use `new` só para trabalho delegado. Trabalho pequeno, acoplado ou ambíguo fica com o lead;
   diga por quê em uma linha.

## Escolher um playbook e adaptar

Antes de planejar, rode `playbook --pick` para ver o perfil ativo, a ordem dos provedores, as
condições de escolha e o resumo dos playbooks. Depois, `playbook <id>` para o escolhido, com tiers
resolvidos e regras de cota e orçamento. Os dois aceitam `--json`. O `playbook` sem argumentos
mostra o arquivo inteiro, só para edição ou onboarding. Veja [playbooks.md](playbooks.md).

1. Escolha um playbook pela `pickOrder`. Um playbook nomeado pelo usuário vence. O nome de uma
   estratégia do runner também vence e mantém o comportamento de onda única descrito abaixo.
   `complete` se adapta por unidade quando nada mais serve.
2. Rode as fases em ordem. `who: lead` é você, nesta sessão. Toda outra fase é uma onda de workers:
   - `runner` é a `strategy` do plano;
   - `workers` é o número de unidades;
   - `readOnly` significa `mode: read-only`;
   - `isolate` dá a cada writer sua própria worktree.
3. Pule uma fase cujo `if` não vale; entre caminhos unidos por `with`, roda o primeiro verdadeiro.
   Confira você mesmo o `gate` de uma fase; se falhar, rode a cadeia `after` uma vez por unidade e
   confira de novo.
4. Resolva o tier de cada worker pelo arquivo de playbooks e pelo perfil ativo, depois aplique a
   primeira regra que casar. Overrides antigos em `policy.modelTiers` ainda vencem quando
   existirem; salve tiers novos só nos playbooks. Passe modelo e esforço explicitamente e mantenha
   trabalho somente leitura num provedor capaz.
5. Diga em uma linha qual playbook escolheu e por quê, antes da primeira onda.

## Vigiar a cota

Nunca deixe um provedor esgotar, sob qualquer objetivo, perfil ou estratégia, pedido ou não. Antes
de cada onda:

1. Rode `usage`.
   - As leituras do Codex vêm dos arquivos de sessão dele; as do Claude, do `rate_limit_event` de
     cada worker Claude; as do Antigravity (`agy`), de `agy -p /usage`.
   - Quando o plano ou você usar o Claude e a leitura dele estiver ausente ou velha (mais de 30
     minutos), rode `usage --refresh-claude`, uma chamada de uma linha ao Haiku.
   - Quando o plano usar o `agy` e a leitura estiver ausente ou velha, rode `usage --refresh-agy`
     (zero tokens). Ele confere `5h` e `week` dos modelos Gemini, `argon` dos modelos Argon e
     `3p-5h`/`3p-week` dos modelos Claude/GPT de terceiros dentro do Antigravity.
2. Leia o `quotaForecast` do `dry-run`: quanto a onda pode consumir de cada janela, aprendido com
   ondas anteriores. Com poucas amostras ele é desconhecido; diga isso em vez de chutar.
3. Se um provedor estiver baixo (no ou acima de `quota.criticalThreshold`) e o próximo na ordem
   rodar o mesmo tier, mova o trabalho e avise em uma linha com os números e o horário do reset:
   "Movendo os scouts para o Codex: seu Claude está em 86% da janela de 5 horas (reset 14:30), o
   Codex em 33%."
   - Aplique `quota.whenSpare` só a provedores por assinatura cuja cota sobrando se perderia,
     respeitando o `billing` em `coding-opus-prices.json` (padrão `subscription`).
   - Mantenha o padrão `spend-strong`; ele nunca vale para cobrança por API, que não tem cota que
     expira.
4. Pare e pergunte antes de lançar quando a única saída de um provedor baixo rebaixar o tier que o
   trabalho precisa, quando todos os provedores estiverem baixos, ou quando a previsão passar de
   `quota.stopAt`. Diga o que o trabalho precisa, o que sobra e quando reseta. Ofereça de duas a
   quatro opções com uma recomendação, usando a ferramenta de pergunta do agente quando houver:
   - esperar o reset;
   - rodar em outro provedor, dizendo o que muda;
   - um tier mais barato só para as unidades que permitem, dizendo o risco;
   - rodar agora só as unidades que cabem;
   - conectar outro CLI ou conta (`setup`);
   - seguir assim mesmo, só com a palavra do usuário, com `run --allow-low-quota`.

   "Esta migração precisa do tier forte (Opus). Seu Claude está em 91% da janela de 5 horas e
   reseta às 14:30, em 2h10; esta onda pode levar uns 15%. 1) esperar até 14:30 (recomendado),
   2) Codex high agora, em 40%, 3) Sonnet só para as duas unidades mecânicas, 4) conectar outro CLI."
5. O `run` recusa uma onda, e pula workers na fila (`failureKind: quota_low`), num provedor no ou
   acima de `quota.stopAt`. Um worker que bateu num limite falha como `rate_limited`. Trate os dois
   como no passo 4.

O provedor fixado pelo usuário, e a resposta dele, vencem estas regras. Quando você mesmo rodar
num provedor baixo, diga isso, mantenha seus turnos curtos e passe trabalho a um provedor com
folga. Se uma leitura estiver velha, diga isso em vez de afirmar o número. Registre cada mudança
com `note "<motivo e leituras>" --kind switch`. Veja a referência para leituras desconhecidas.

## Mudar o rumo

Entre ondas, adapte o plano aos resultados ou ao pedido do usuário:
- pule, repita ou acrescente fases;
- faça uma etapa você mesmo;
- troque provedores, modelos ou orçamento;
- troque de playbook ou planeje ondas freeform.

O pedido do usuário vence o playbook.

Registre cada mudança em uma linha antes da próxima onda, para o relatório explicá-la:

| Mudança | Comando |
| --- | --- |
| pular uma fase | `note "<fase>: <motivo>" --kind skip` |
| acrescentar uma fase ou um worker | `note "<o quê>: <motivo>" --kind add` |
| trocar de playbook, ou ir para freeform | `note "para <playbook ou freeform>: <motivo>" --kind switch` |
| fazer uma etapa você mesmo | `note "<o que fez>" --kind lead` |
| qualquer outra | `note "<o que mudou e por quê>"` |

Uma onda freeform é um plano comum: deixe `playbook` de fora e dê à `phase` o nome do que a onda
faz. Pare uma onda que deixou de ser útil com `stop`; workers que já rodaram mantêm os resultados.

## Orquestrar

1. Delegue quando o custo extra compensa. As estratégias `economy` e `rescue` permitem um worker.
2. Leia [routing.md](routing.md) para as estratégias que cada onda pode usar: `auto`, `economy`,
   `balanced`, `quality`, `research`, `parallel` ou `rescue`. Uma estratégia explícita no pedido
   atual vence as preferências salvas. Trate a palavra depois de uma invocação explícita da skill
   como playbook ou estratégia quando casar com um nome.
3. Rode `probe` e use só provedores disponíveis. Leia [providers.md](providers.md) ao usar Agy,
   Kimi, retomar um worker ou acrescentar um CLI próprio.
4. Dê aos writers posse disjunta. Use worktree isolada para writers concorrentes; um único writer
   pode usar o checkout principal quando as mudanças existentes do usuário forem preservadas.
5. Crie um plano JSON versão 1 fora do repositório.
   - `scaffold <estratégia> --workers N` imprime um esqueleto válido para preencher.
   - Use `strategy`. Use `provider: auto` só quando preferências salvas de troca de provedor
     puderem decidir o provedor.
   - Defina o `budget` do plano quando uma onda precisar de limites rígidos. Rode `budget` quando um
     limite cobrir todas as ondas da orquestração.
   - Dê ao plano um `title`, o `playbook` e a `phase` que ele roda e um `why` de uma linha. Dê a
     cada worker um `why` curto com seu papel e o motivo do modelo; o relatório os mostra.
   - Escreva cada tarefa como [briefs.md](briefs.md) descreve.
   - Workers Codex e Claude começam sem servidores MCP; use `mcp: true` só num brief que precise.
   - Valide o plano contra [plan.schema.json](../schemas/plan.schema.json) ou com `validate`.
6. Rode `models`. Para um preço necessário ausente ou velho (mais de 60 dias), consulte só a página
   oficial de preços do fornecedor, avise o usuário e salve a entrada com `source` e `asOf` em
   `coding-opus-prices.json`, ao lado da política; nunca chute. Veja [operations.md](operations.md).
   - Faça a prévia com `dry-run`: estratégia, provedores, concorrência, tokens de prompt, orçamento,
     `listPriceEstimateUsd` (uma estimativa) e `quotaForecast`.
   - Corrija os avisos (modelos padrão, provedores baixos, linha de parada) ou explique por que não
     se aplicam.
   - Antes da primeira onda de uma orquestração não pedida com várias ondas ou vários workers,
     mostre a prévia em poucas linhas e espere o ok, como em "Ficar ligado". Um objetivo de modo
     sozinho nunca é essa permissão; cobrança por API e cota desconhecida ou velha no passivo ainda
     exigem aprovação.
   - Quando o usuário quiser ver a arquitetura, salve a prévia com `dry-run --write` e desenhe com
     `scripts/diagram.py`, como em [diagram.md](diagram.md). Mostre o SVG antes de rodar workers e
     use só rotas de proxy confirmadas.
7. Inicie a orquestração uma vez com `new "<título>" --playbook <nome> --why "<uma linha>"
   --lead <seu CLI>:<seu modelo>`, como `--lead claude:opus`. Isso cria a pasta, a torna atual e
   registra você como lead no relatório.
   - Inicie cada onda com `run <plano> --in current --detach` e chame `wait [--timeout N]` (no
     máximo 540 segundos) até `done` ser verdadeiro; ele também devolve o status. O Claude Code
     pode usar o modo em segundo plano da ferramenta de shell.
   - Nunca rode uma onda em primeiro plano numa ferramenta de shell com timeout curto.
   - Runners interrompidos são detectados; `stop` também mata workers que sobraram.
   - O runner limita concorrência e orçamentos e deixa worktrees para revisão. Ele usa arrays de
     argumentos; um shim `.cmd` ilegível roda pelo `cmd.exe` só com as restrições de argumento de
     [providers.md](providers.md).
8. Leia o `state.json`, cada `outcome.json` (primeiro `workerStatus`, `failureKind` e
   `readOnlyViolation`), o `result.md` conciso e os `diff.patch` dos writers. Não carregue logs
   brutos a menos que esteja diagnosticando uma falha.
9. Integre as mudanças revisadas e rode a validação no checkout combinado. Marque o resultado de
   cada worker com `mark` (`used`, `partial` ou `discarded`). Remova só dados temporários e
   worktrees cujas mudanças úteis foram tratadas. `mark <worker> used` acha o worker na
   orquestração atual.
10. Feche com `report` e mostre a saída: trabalho, motivos, mudanças, gasto relatado e o que falta.
    Use-o para perguntas de custo; sem pasta, vale a orquestração atual.

Para exemplos de plano, semântica dos resultados, cópia seletiva de estado sujo e limpeza, leia
[operations.md](operations.md).

## Contrato do worker

Cada brief precisa dizer:
- o objetivo da tarefa e a definição de pronto;
- os caminhos que o worker possui;
- as regras relevantes do repositório e as checagens;
- as mudanças proibidas e a entrega desejada.

O runner acrescenta automaticamente estas invariantes:

- não delegar, não criar subagentes e não lançar outro CLI de agente;
- não fazer commit, push, tag, release nem abrir pull request;
- devolver um resultado conciso, evidência, arquivos alterados, checagens, falhas e riscos;
- terminar com uma linha `STATUS:` (`DONE`, `DONE_WITH_CONCERNS`, `BLOCKED` ou `NEEDS_CONTEXT`) e
  uma linha `SUMMARY:`.

O runner bloqueia subagentes do Codex e do Claude quando os CLIs anunciam as flags necessárias.
Todo worker roda com `CODING_OPUS_WORKER=1`, e o runner recusa orquestração aninhada. Um worker com
acesso ao shell pode remover essa proteção, que é só indicativa. O status e o resumo de um worker
são alegações: confira o diff. Para briefs de revisão, validação, juiz e painel, siga
[briefs.md](briefs.md).

**Segurança dos workers.**
- Não ponha segredos em planos, tarefas, argumentos ou logs.
- Não use flags de bypass, a menos que o usuário peça explicitamente essa perda de isolamento.
  Workers próprios exigem `allowUnsafe: true` para flags inseguras conhecidas e
  `trust.allowUnsafe: true` na política salva do usuário.
- Workers Kimi ou próprios somente leitura são recusados: use Codex, Claude ou Agy, ou
  `mode: write` com `isolate: true`.
- O `cwd` de um worker precisa ficar no checkout que o invocou (ou numa pasta fora do git), salvo
  se o usuário permitir em `trust.cwdRoots`. Arquivos de tarefa ficam dentro da pasta do plano ou
  da orquestração.
- O lead nunca escreve `trust` por conta própria; só o usuário, ou o lead depois que o usuário
  pedir explicitamente essa mudança exata.

## Disciplina de tokens

- Trate frases como "estou com poucos tokens", "poupe cota" ou "economize" como estratégia
  `economy` na execução atual. Persista com `policy set economy` só quando o usuário pedir um
  padrão permanente.
- O `budget` de um plano transforma economia em limite rígido: chamadas de worker, tempo, tokens
  de prompt estimados, tokens relatados e custo relatado. Use `dry-run` para ver a estimativa antes
  de rodar.
  - Só o Claude relata custo, então `maxReportedTokens` limita os outros.
  - Leituras de cache são relatadas à parte e ficam fora de `maxReportedTokens` e da divisão de
    tokens do relatório.
- Mais workers aumentam o uso total. `auto` precisa escolher não orquestrar quando a tarefa sai
  mais barata direto; se a orquestração foi pedida explicitamente, ele escolhe a menor estratégia
  válida.
- Aponte para arquivos em vez de copiá-los; leia resultados concisos e diffs, não transcrições.
- Reutilize `sessionId` só para continuações focadas. Use workers baratos para descoberta limitada
  e trabalho mecânico; deixe ambiguidade e integração com o lead ou um revisor capaz.
- Um worker que falhou pode ser repetido uma vez, só quando o erro tiver uma causa específica e
  corrigível.

Não faça commit, push, tag, release nem abra pull request sem pedido explícito do usuário.

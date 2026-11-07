export const inspector: Record<string, string> = {
  // Inspector.svelte
  "Save Inspector warning": "Aviso do Inspetor de Save",
  "Save Inspector edits can break a save.":
    "Edições no Inspetor de Save podem corromper um save.",
  "Keep an independent backup and change only fields you understand. Raw structural edits may make the save unreadable or cause problems in the game.":
    "Mantenha um backup independente e altere apenas campos que você entende. Edições estruturais brutas podem tornar o save ilegível ou causar problemas no jogo.",
  "Dismiss Save Inspector warning": "Dispensar aviso do Inspetor de Save",
  Dismiss: "Dispensar",
  "Save structure": "Estrutura do save",
  "Resize inspector panes": "Redimensionar painéis do inspetor",
  "Drag to resize panes; press Home to reset":
    "Arraste para redimensionar os painéis; pressione Home para redefinir",
  "Record details": "Detalhes do registro",
  "Record path": "Caminho do registro",
  "Show raw structure": "Mostrar estrutura bruta",
  Handle: "Identificador",
  "Wire type": "Tipo serializado",
  Value: "Valor",
  "Record value": "Valor do registro",
  "Apply value": "Aplicar valor",
  "Go to referenced object": "Ir para o objeto referenciado",
  "Target handle": "Identificador de destino",
  "Retarget reference": "Redirecionar referência",
  "Add a field or entry": "Adicionar um campo ou entrada",
  Name: "Nome",
  Type: "Tipo",
  "Add scalar / array": "Adicionar escalar / array",
  Template: "Modelo",
  "Choose a supported type": "Escolha um tipo suportado",
  "Create from template": "Criar a partir do modelo",
  "Duplicate subtree": "Duplicar subárvore",
  "Sibling index": "Índice entre irmãos",
  Move: "Mover",
  Remove: "Remover",
  "Duplication preserves game identifiers, including SGuid.":
    "A duplicação preserva identificadores do jogo, incluindo o SGuid.",
  "Select a record in the tree to inspect or edit it.":
    "Selecione um registro na árvore para inspecioná-lo ou editá-lo.",
  // SearchPane.svelte
  "Search mode": "Modo de busca",
  Simple: "Simples",
  Advanced: "Avançada",
  "Search save structure": "Buscar na estrutura do save",
  "Search names, values, classes and paths":
    "Busque nomes, valores, classes e caminhos",
  "Enter an advanced query": "Digite uma consulta avançada",
  "Clear search": "Limpar busca",
  Filters: "Filtros",
  Path: "Caminho",
  "Data types": "Tipos de dados",
  "Own class": "Classe própria",
  "Nearest class": "Classe mais próxima",
  "Any ancestor": "Qualquer ancestral",
  Minimum: "Mínimo",
  Maximum: "Máximo",
  "Query syntax": "Sintaxe de consulta",
  "Spaces mean AND,": "Espaços significam AND,",
  "means OR,": "significa OR,",
  "means NOT, and parentheses group expressions.":
    "significa NOT, e parênteses agrupam expressões.",
  "Qualifiers:": "Qualificadores:",
  Use: "Use",
  and: "e",
  or: "ou",
  "as wildcards, or": "como curingas, ou",
  "for an exact text match. Numeric comparisons use":
    "para uma correspondência exata de texto. Comparações numéricas usam",
  "and similar operators.": "e operadores semelhantes.",
  "to match related stored records.":
    "para corresponder a registros armazenados relacionados.",
  "Case sensitive": "Diferenciar maiúsculas e minúsculas",
  "Syntax help": "Ajuda de sintaxe",
  Search: "Buscar",
  "Inspector search view": "Visualização de busca do inspetor",
  "Results ({count})": "Resultados ({count})",
  Structure: "Estrutura",
  "{count} matches": "{count} correspondências",
  "Searching…": "Buscando…",
  Incomplete: "Incompleta",
  Complete: "Completa",
  "Preparing search…": "Preparando busca…",
  "Search results": "Resultados da busca",
  "{field} match": "correspondência de {field}",
  Reveal: "Revelar",
  "Loading…": "Carregando…",
  // TreeNode.svelte
  "Collapse {name}": "Recolher {name}",
  "Expand {name}": "Expandir {name}",
  "{count} fields": "{count} campos",
  "Load from beginning": "Carregar do início",
  "Load more ({count})": "Carregar mais ({count})",
  // widgets/SGuid.svelte
  GUID: "GUID",
  "Enter a GUID in the form 00000000-0000-0000-0000-000000000000.":
    "Digite um GUID no formato 00000000-0000-0000-0000-000000000000.",
  "Apply GUID": "Aplicar GUID",
  // widgets/Vector.svelte
  "Vector {axis}": "Vetor {axis}",
  "Apply vector": "Aplicar vetor",
  // SearchHelpDialog.svelte
  "Inspector search syntax": "Sintaxe de busca do inspetor",
  "Simple search": "Busca simples",
  "Simple search checks a record's value, name, class, wire type and logical path. Results rank direct value matches first, followed by names, classes, types and paths. Each result states which field matched.":
    "A busca simples verifica o valor, o nome, a classe, o tipo serializado e o caminho lógico de um registro. Os resultados priorizam correspondências diretas de valor, seguidas de nomes, classes, tipos e caminhos. Cada resultado indica qual campo correspondeu.",
  "Substring matching is literal, so":
    "A correspondência de substring é literal, então",
  "also occurs inside": "também ocorre dentro de",
  "in Advanced mode, or the Value filter in Simple mode, when only stored values should match.":
    "no modo Avançada, ou o filtro Valor no modo Simples, quando apenas valores armazenados devem corresponder.",
  "Operators and text": "Operadores e texto",
  Syntax: "Sintaxe",
  Meaning: "Significado",
  space: "espaço",
  "AND. Every adjacent expression must match.":
    "AND. Toda expressão adjacente deve corresponder.",
  "OR. Either expression may match.":
    "OR. Qualquer uma das expressões pode corresponder.",
  "NOT. Excludes matching records.":
    "NOT. Exclui registros correspondentes.",
  "Groups expressions. Grouping is evaluated before NOT, AND and OR.":
    "Agrupa expressões. O agrupamento é avaliado antes de NOT, AND e OR.",
  "Treats spaces, operators and wildcards as literal text. Use":
    "Trata espaços, operadores e curingas como texto literal. Use",
  "inside quotes.": "dentro das aspas.",
  "Matches zero or more characters. A wildcard pattern must match the whole selected field.":
    "Corresponde a zero ou mais caracteres. Um padrão com curinga deve corresponder ao campo selecionado inteiro.",
  "Matches exactly one character.": "Corresponde a exatamente um caractere.",
  "Matches the entire selected text field. It can qualify a group of alternatives.":
    "Corresponde ao campo de texto selecionado inteiro. Pode qualificar um grupo de alternativas.",
  Qualifiers: "Qualificadores",
  "The record's field or entry name.":
    "O nome do campo ou da entrada do registro.",
  "The displayed scalar value. This is textual matching, even for":
    "O valor escalar exibido. A correspondência é textual, mesmo para",
  "The wire data type, such as": "O tipo de dado serializado, como",
  "Aliases are": "Os aliases são",
  "The record's own serialized class. A string field normally has no own class.":
    "A classe serializada própria do registro. Um campo string normalmente não tem classe própria.",
  "The nearest typed object containing the record, excluding the record itself.":
    "O objeto tipado mais próximo que contém o registro, excluindo o próprio registro.",
  "Any typed object containing the record at any depth, excluding the record itself.":
    "Qualquer objeto tipado que contenha o registro em qualquer profundidade, excluindo o próprio registro.",
  "The logical route to the record. It includes field names, array positions and followed internal references.":
    "A rota lógica até o registro. Inclui nomes de campos, posições de array e referências internas seguidas.",
  "Class names may be short": "Nomes de classe podem ser curtos",
  "namespace-qualified": "qualificados por namespace",
  "or fully serialized with an assembly name. Search uses actual serialized identities and does not infer inheritance.":
    "ou totalmente serializados com um nome de assembly. A busca usa as identidades serializadas reais e não infere herança.",
  "Numeric comparisons": "Comparações numéricas",
  "Negative numbers, decimals and scientific notation are accepted. Multiple comparisons form a range. Integer comparisons remain exact for 64-bit values. A single":
    "Números negativos, decimais e notação científica são aceitos. Múltiplas comparações formam um intervalo. Comparações de inteiros permanecem exatas para valores de 64 bits. Um único",
  "is numeric equality; use": "é igualdade numérica; use",
  "for exact text.": "para texto exato.",
  "The result has at least one direct stored child that matches the enclosed expression.":
    "O resultado tem pelo menos um filho armazenado direto que corresponde à expressão interna.",
  "The result has a matching stored descendant at any depth. Direct children are included.":
    "O resultado tem um descendente armazenado correspondente em qualquer profundidade. Filhos diretos estão incluídos.",
  "The result's direct stored parent matches the enclosed expression.":
    "O pai armazenado direto do resultado corresponde à expressão interna.",
  "Structural predicates change which record is returned: the enclosed expression identifies a related record while the outer expression filters the result itself. They may contain":
    "Predicados estruturais mudam qual registro é retornado: a expressão interna identifica um registro relacionado enquanto a expressão externa filtra o próprio resultado. Eles podem conter",
  "numeric comparisons and nested structural predicates. Route-relative qualifiers such as":
    "comparações numéricas e predicados estruturais aninhados. Qualificadores relativos à rota, como",
  "belong outside them.": "devem ficar fora deles.",
  Examples: "Exemplos",
  "Use query": "Usar consulta",
  "Reference paths and limits": "Caminhos de referência e limites",
  "Search follows internal object references and evaluates all conditions on the same logical route. A record reachable by several routes is shown once, using its first matching route. Cycles stop before revisiting an object. If a depth or traversal limit is reached, the result state says Incomplete and explains why.":
    "A busca segue referências internas de objetos e avalia todas as condições na mesma rota lógica. Um registro alcançável por várias rotas é mostrado uma vez, usando sua primeira rota correspondente. Ciclos param antes de revisitar um objeto. Se um limite de profundidade ou travessia for atingido, o estado do resultado indica Incompleta e explica o motivo.",
  Close: "Fechar",
  // SearchHelpDialog.svelte example names and explanations
  "Inventory item string IDs": "IDs string de itens do inventário",
  "Find string fields named id below an Item anywhere on a logical path containing playerInventory, then require iron in the value.":
    "Encontra campos string chamados id abaixo de um Item em qualquer lugar de um caminho lógico contendo playerInventory, exigindo iron no valor.",
  "Numeric range": "Intervalo numérico",
  "Find integer or floating-point records whose numeric value is between 10 and 100, including both endpoints.":
    "Encontra registros inteiros ou de ponto flutuante cujo valor numérico está entre 10 e 100, incluindo ambos os extremos.",
  "Alternative item values outside equipment":
    "Valores alternativos de itens fora de equipment",
  "Match either material name, then exclude records reached through an equipment path.":
    "Corresponde a qualquer um dos nomes de material e depois exclui registros alcançados por um caminho de equipment.",
  "Exact field with alternative values": "Campo exato com valores alternativos",
  "Find fields named exactly worldId whose complete value is either Prison or RuinedTemple. The case-sensitive option also applies to exact matches.":
    "Encontra campos chamados exatamente worldId cujo valor completo é Prison ou RuinedTemple. A opção de diferenciar maiúsculas e minúsculas também se aplica a correspondências exatas.",
  "Object with a direct world field": "Objeto com um campo de mundo direto",
  "Return each object or container that has a direct child field named worldId whose value is exactly Prison or RuinedTemple.":
    "Retorna cada objeto ou contêiner que tem um campo filho direto chamado worldId cujo valor é exatamente Prison ou RuinedTemple.",
  "Object containing a nested world field":
    "Objeto contendo um campo de mundo aninhado",
  "Return containers with that worldId field anywhere below them. Add a target filter such as class:WgoData when only one kind of containing object should be returned.":
    "Retorna contêineres com esse campo worldId em qualquer lugar abaixo deles. Adicione um filtro de destino como class:WgoData quando apenas um tipo de objeto contêiner deve ser retornado.",
  "Elements below PalaceSewer": "Elementos abaixo de PalaceSewer",
  "Return direct children of a parent that identifies itself through a direct id field equal to PalaceSewer, excluding the id field itself.":
    "Retorna filhos diretos de um pai que se identifica por um campo id direto igual a PalaceSewer, excluindo o próprio campo id.",
  "Fields directly owned by Item": "Campos pertencentes diretamente a Item",
  "Find positive numeric fields whose nearest containing typed object is Item. Nested typed objects inside an Item do not match this owner filter.":
    "Encontra campos numéricos positivos cujo objeto tipado contêiner mais próximo é Item. Objetos tipados aninhados dentro de um Item não correspondem a esse filtro de owner.",
  "Anything nested below an Item": "Qualquer coisa aninhada abaixo de um Item",
  "Find id fields at any depth below an Item, including fields inside other typed objects nested within that Item.":
    "Encontra campos id em qualquer profundidade abaixo de um Item, incluindo campos dentro de outros objetos tipados aninhados nesse Item.",
  "Literal operator characters": "Caracteres literais de operadores",
  "Quotes make spaces and operator characters literal. Wildcards inside quotes are literal too.":
    "Aspas tornam espaços e caracteres de operadores literais. Curingas dentro de aspas também são literais.",
};

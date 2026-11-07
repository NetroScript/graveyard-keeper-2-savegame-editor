export const zombies: Record<string, string> = {
  // Zombies.svelte — section intro and list
  Zombies: "Zumbis",
  "Customize each worker’s body, equipment, talents and appearance.":
    "Personalize o corpo, o equipamento, os talentos e a aparência de cada trabalhador.",
  "Optimize body": "Otimizar corpo",
  "Loading zombie definitions and save state…":
    "Carregando definições de zumbis e estado do save…",
  "No zombies were found in this save.":
    "Nenhum zumbi foi encontrado neste save.",
  "{name} portrait": "Retrato de {name}",
  "{count} talent slot available": "{count} slot de talento disponível",
  "{count} talent slots available": "{count} slots de talento disponíveis",
  "Not currently assigned": "Sem atribuição no momento",
  "Zombie details": "Detalhes do zumbi",
  Overview: "Visão geral",
  "Body & equipment": "Corpo e equipamento",
  Talents: "Talentos",
  Appearance: "Aparência",
  // Overview panel
  Name: "Nome",
  "Zombie name": "Nome do zumbi",
  "Apply name": "Aplicar nome",
  "Technology points": "Pontos de Tecnologia",
  red: "vermelho",
  green: "verde",
  blue: "azul",
  "Set all to 9,999": "Definir tudo para 9.999",
  "Apply points": "Aplicar pontos",
  Assignment: "Atribuição",
  Role: "Função",
  Location: "Localização",
  "Not assigned": "Sem atribuição",
  Workstation: "Estação de trabalho",
  Assigned: "Atribuído",
  None: "Nenhuma",
  "Free talent slots": "Slots de talento livres",
  "Quick improvement": "Melhoria rápida",
  "Applies the strongest available organs, body treatments and collar.":
    "Aplica os órgãos, tratamentos corporais e coleira mais fortes disponíveis.",
  "Advanced save data": "Dados avançados do save",
  "Zombie record": "Registro do zumbi",
  "Body record": "Registro do corpo",
  "Open in Save Inspector": "Abrir no Inspetor de Save",
  "Unique ID": "ID único",
  // Body & equipment panel
  "{count}/6 body pockets used": "{count}/6 bolsos do corpo usados",
  Organs: "Órgãos",
  Brain: "Cérebro",
  Heart: "Coração",
  Intestines: "Intestinos",
  Skull: "Caveira",
  Bones: "Ossos",
  Skin: "Pele",
  "Body pockets": "Bolsos do corpo",
  "Slot {number}": "Slot {number}",
  Equipment: "Equipamento",
  Collar: "Coleira",
  Armor: "Armadura",
  Tool: "Ferramenta",
  "Work storage": "Armazenamento de trabalho",
  "Current cargo": "Carga atual",
  "Items currently being transported. Large cargo and its reserved slot are shown but remain game-managed.":
    "Itens sendo transportados no momento. Cargas grandes e seu slot reservado são exibidos, mas continuam gerenciados pelo jogo.",
  "{used}/{capacity} slots used": "{used}/{capacity} slots usados",
  "Reserved for large cargo": "Reservado para carga grande",
  "Cargo {number}": "Carga {number}",
  "Empty cargo slot {number}": "Slot de carga vazio {number}",
  "This is live work state. The game will deposit or replace it as the current task progresses.":
    "Este é o estado de trabalho em tempo real. O jogo vai depositar ou substituir o item conforme a tarefa atual avança.",
  "In transit": "Em trânsito",
  "Gardeners use their body pockets while working. Edit them above.":
    "Jardineiros usam os bolsos do corpo enquanto trabalham. Edite-os acima.",
  "This worker uses the inventory of its assigned workstation. Edit that container from the Inventories section.":
    "Este trabalhador usa o inventário da estação de trabalho atribuída. Edite esse contêiner na seção Inventários.",
  "This worker has no separate work inventory.":
    "Este trabalhador não tem um inventário de trabalho separado.",
  // Talents panel
  "Use technology points": "Usar pontos de Tecnologia",
  "Deduct costs when unlocking individual talents.":
    "Desconta os custos ao desbloquear talentos individuais.",
  "Optimize for {branch}": "Otimizar para {branch}",
  "{branch} needs {needed} talent slots, but the strongest available body provides {available}.":
    "{branch} precisa de {needed} slots de talento, mas o corpo mais forte disponível fornece {available}.",
  "{name} is now optimized for {branch}.":
    "{name} agora está otimizado para {branch}.",
  "The strongest available body parts and collar have been applied.":
    "As partes do corpo e a coleira mais fortes disponíveis foram aplicadas.",
  // Appearance panel
  "Zombie appearance preview": "Prévia da aparência do zumbi",
  "This save contains a body style that this worker cannot load safely. Applying this appearance will restore a valid body while keeping any compatible head settings.":
    "Este save contém um estilo de corpo que este trabalhador não consegue carregar com segurança. Aplicar esta aparência vai restaurar um corpo válido mantendo as configurações de cabeça compatíveis.",
  Body: "Corpo",
  "Previous body": "Corpo anterior",
  "Next body": "Próximo corpo",
  Head: "Cabeça",
  "Previous head": "Cabeça anterior",
  "Next head": "Próxima cabeça",
  "Body palette": "Paleta do corpo",
  "Previous body palette": "Paleta de corpo anterior",
  "Next body palette": "Próxima paleta de corpo",
  "Head palette": "Paleta da cabeça",
  "Previous head palette": "Paleta de cabeça anterior",
  "Next head palette": "Próxima paleta de cabeça",
  "Apply appearance": "Aplicar aparência",
  // Item dialog titles
  "Choose {label}": "Escolher {label}",
  "Choose body item": "Escolher item para o corpo",
  "Choose porter cargo": "Escolher carga do carregador",
  // ZombieItemDialog.svelte — tool type groups
  "Fishing rods": "Varas de pescar",
  "Alchemy kits": "Kits de alquimia",
  Axes: "Machados",
  Hammers: "Martelos",
  Pickaxes: "Picaretas",
  Shovels: "Pás",
  "Crafting tools": "Ferramentas de criação",
  "Surgical kits": "Kits cirúrgicos",
  Books: "Livros",
  Talismans: "Talismãs",
  Hands: "Mãos",
  // ZombieItemDialog.svelte — groups and notes
  "Best for {talent}": "Melhores para {talent}",
  "current specialty": "a especialidade atual",
  "Higher ability levels improve work that uses this specialty.":
    "Níveis de habilidade mais altos melhoram o trabalho que usa esta especialidade.",
  "Normal armor": "Armadura normal",
  "Higher armor levels provide stronger combat protection.":
    "Níveis de armadura mais altos oferecem proteção de combate mais forte.",
  "Debug and NPC armor": "Armadura de debug e de NPC",
  "Not normally obtainable. These entries may not behave correctly on a worker.":
    "Normalmente não obtenível. Estas entradas podem não funcionar corretamente em um trabalhador.",
  "Body treatments": "Tratamentos corporais",
  "Treatments can add or remove skulls and occupy body-inventory slots.":
    "Tratamentos podem adicionar ou remover caveiras e ocupam slots do inventário corporal.",
  "Skull-adding items": "Itens que adicionam caveiras",
  "These items increase the zombie's red or white skull total.":
    "Estes itens aumentam o total de caveiras vermelhas ou brancas do zumbi.",
  "Other items": "Outros itens",
  "Cargo items": "Itens de carga",
  "Choose an item and the amount this porter should carry.":
    "Escolha um item e a quantidade que este carregador deve levar.",
  Variants: "Variantes",
  Choices: "Opções",
  // ZombieItemDialog.svelte — item metadata
  "Ability +{bonus}": "Habilidade +{bonus}",
  "No ability bonus": "Sem bônus de habilidade",
  "Level {level} · not normally obtainable":
    "Nível {level} · normalmente não obtenível",
  Minimal: "mínima",
  Basic: "básica",
  Good: "boa",
  Strong: "forte",
  Strongest: "máxima",
  Unknown: "desconhecida",
  "Armor level {level} · {protection} normal protection":
    "Nível de armadura {level} · proteção normal {protection}",
  Item: "Item",
  // ZombieItemDialog.svelte — form
  Search: "Buscar",
  Variant: "Variante",
  "Quality {quality}": "Qualidade {quality}",
  Amount: "Quantidade",
  "Cargo amount": "Quantidade de carga",
  "Stack size: {size}": "Tamanho da pilha: {size}",
  "Clear slot": "Limpar slot",
  Cancel: "Cancelar",
  Apply: "Aplicar",
  // ZombieSlot.svelte
  "Change {label}: {name}": "Alterar {label}: {name}",
  "Add {label}": "Adicionar {label}",
  "Clear {label}": "Limpar {label}",
  // SkullValue.svelte
  "{red} red skulls, {white} white skulls":
    "{red} caveiras vermelhas, {white} caveiras brancas",
};

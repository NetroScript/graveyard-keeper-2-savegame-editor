export const progression: Record<string, string> = {
  // Shared (Technologies, Inspirations, InfoPopover)
  "State": "Estado",
  "Unlocked": "Desbloqueado",
  "Locked": "Bloqueado",
  "Unlocks": "Desbloqueios",
  "Prerequisites": "Pré-requisitos",
  "Locked {name}.": "Bloqueou {name}.",
  "{name} is available from the start. The game unlocks it again when the save loads.":
    "{name} está disponível desde o início. O jogo o desbloqueia de novo quando o save carrega.",
  "Available from the start. The game unlocks it again when the save loads, so it cannot be locked.":
    "Disponível desde o início. O jogo o desbloqueia de novo quando o save carrega, então não dá para bloqueá-lo.",

  // Technologies
  "Technologies": "Tecnologias",
  "Hover for details. Selecting a locked technology unlocks it and all of its prerequisites; selecting an unlocked technology locks it again, together with technologies that depend on it. Technologies that were already unlocked when the save was opened refund their technology points when locked and cost them again when unlocked; everything else is free.":
    "Passe o mouse para ver os detalhes. Selecionar uma tecnologia bloqueada desbloqueia ela e todos os pré-requisitos dela; selecionar uma tecnologia desbloqueada a bloqueia de novo, junto com as tecnologias que dependem dela. Tecnologias que já estavam desbloqueadas quando o save foi aberto reembolsam seus pontos de tecnologia ao serem bloqueadas e os cobram de novo ao serem desbloqueadas; todo o resto é de graça.",
  "Technology trees": "Árvores de tecnologia",
  "{tab} technology tree": "Árvore de tecnologia {tab}",
  "Technology": "Tecnologia",
  "Character reputation": "Reputação de personagem",
  "District reputation": "Reputação de distrito",
  "Revealed by game progress": "Revelada pelo progresso do jogo",
  "Type": "Tipo",
  "Requirement": "Requisito",
  "Other game effects": "Outros efeitos do jogo",
  "Crafted at": "Fabricado em",
  "Crafted at: {list}": "Fabricado em: {list}",
  "red": "vermelho",
  "green": "verde",
  "blue": "azul",
  "Unlocked {name}. Spent {points} technology points.":
    "Desbloqueou {name}. Pontos de tecnologia gastos: {points}.",
  "Locked {name} and 1 dependent technology.":
    "Bloqueou {name} e 1 tecnologia dependente.",
  "Locked {name} and {count} dependent technologies.":
    "Bloqueou {name} e {count} tecnologias dependentes.",
  "Refunded {points} technology points.":
    "Pontos de tecnologia reembolsados: {points}.",
  "This is a {kind} gate. Unlocking it bypasses that requirement without changing quests or reputation.":
    "Este é um bloqueio de {kind}. Desbloqueá-lo ignora esse requisito sem alterar missões ou reputação.",
  "The game normally reveals this technology through game or quest progress. Unlocking it does not mark that quest as completed.":
    "O jogo normalmente revela esta tecnologia pelo progresso do jogo ou das missões. Desbloqueá-la não marca essa missão como concluída.",
  "The game normally applies 1 additional side effect when this technology unlocks. The editor preserves story state and does not execute game scripts, so locking it again does not reverse effects the game already applied.":
    "O jogo normalmente aplica 1 efeito colateral adicional quando esta tecnologia é desbloqueada. O editor preserva o estado da história e não executa scripts do jogo, então bloqueá-la de novo não reverte efeitos que o jogo já aplicou.",
  "The game normally applies {count} additional side effects when this technology unlocks. The editor preserves story state and does not execute game scripts, so locking it again does not reverse effects the game already applied.":
    "O jogo normalmente aplica {count} efeitos colaterais adicionais quando esta tecnologia é desbloqueada. O editor preserva o estado da história e não executa scripts do jogo, então bloqueá-la de novo não reverte efeitos que o jogo já aplicou.",
  "Unlock with prerequisites": "Desbloquear com pré-requisitos",
  "Lock with dependent technologies": "Bloquear com tecnologias dependentes",
  "Select a technology to see its description, prerequisites and rewards.":
    "Selecione uma tecnologia para ver a descrição, os pré-requisitos e as recompensas.",
  "Loading technology definitions and save state…":
    "Carregando as definições de tecnologia e o estado do save…",

  // Inspirations
  "Inspirations": "Inspirações",
  "Edit inspiration progress and reveal the full perk tree for each talent.":
    "Edite o progresso das inspirações e revele a árvore de perks completa de cada talento.",
  "Inspiration categories": "Categorias de inspiração",
  "Building": "Construção",
  "Metallurgy": "Metalurgia",
  "Farming": "Agricultura",
  "Theology": "Teologia",
  "Anatomy": "Anatomia",
  "Experience": "Experiência",
  "Inspiration level": "Nível de inspiração",
  "Perk points": "Pontos de perk",
  "Mastery": "Maestria",
  "Unlocked levels": "Níveis desbloqueados",
  "Current goal": "Meta atual",
  "Progress": "Progresso",
  "{name} progress": "Progresso de {name}",
  "{name}: {count} unlocked levels": "{name}: {count} níveis desbloqueados",
  "Apply": "Aplicar",
  "Perks": "Perks",
  "Point cost": "Custo em pontos",
  "Hover for perk details. Selecting a locked perk unlocks it and all prerequisites; selecting an unlocked perk locks it again, together with perks that depend on it. Perks that were already unlocked when the save was opened refund their perk points when locked and cost them again when unlocked; everything else is free.":
    "Passe o mouse para ver os detalhes do perk. Selecionar um perk bloqueado desbloqueia ele e todos os pré-requisitos; selecionar um perk desbloqueado o bloqueia de novo, junto com os perks que dependem dele. Perks que já estavam desbloqueados quando o save foi aberto reembolsam seus pontos de perk ao serem bloqueados e os cobram de novo ao serem desbloqueados; todo o resto é de graça.",
  "Unlocked {name}. Spent 1 perk point.":
    "Desbloqueou {name}. Gastou 1 ponto de perk.",
  "Unlocked {name}. Spent {points} perk points.":
    "Desbloqueou {name}. Gastou {points} pontos de perk.",
  "Locked {name} and 1 dependent perk.": "Bloqueou {name} e 1 perk dependente.",
  "Locked {name} and {count} dependent perks.":
    "Bloqueou {name} e {count} perks dependentes.",
  "Refunded 1 perk point.": "Reembolsou 1 ponto de perk.",
  "Refunded {points} perk points.": "Reembolsou {points} pontos de perk.",
  "Loading inspiration definitions and save state…":
    "Carregando as definições de inspiração e o estado do save…",

  // InfoPopover
  "Materials": "Materiais",
};

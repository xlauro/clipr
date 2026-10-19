use std::collections::HashMap;
use std::sync::LazyLock;

static DICTIONARY: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    let mut map = HashMap::new();

    // 💰 Dinheiro / Riqueza / Lucro
    register(&mut map, "💰", &[
        "dinheiro", "grana", "lucro", "lucros", "lucrativo", "lucrativa", "money", "cash",
        "rico", "rica", "riqueza", "milionario", "milionaria", "bilionario", "bilionaria",
        "faturamento", "faturar", "salario", "venda", "vendas", "vender", "dolar", "dollar",
        "bitcoin", "cripto", "pix", "ganho", "ganhos", "renda", "patrimonio",
    ]);

    // 🔥 Fogo / Viralidade / Hype / Sucesso
    register(&mut map, "🔥", &[
        "fogo", "viral", "viralizar", "top", "incrivel", "incriveis", "sucesso", "bombando",
        "explodiu", "explodir", "hot", "fire", "hype", "insano", "insana", "brabo", "braba",
        "brutal", "sensacional", "surreal", "estourou", "bombou",
    ]);

    // 💡 Ideia / Insight / Segredo / Dica
    register(&mut map, "💡", &[
        "ideia", "insight", "segredo", "segredos", "dica", "dicas", "hack", "hacks",
        "truque", "truques", "mente", "pensar", "sacada", "solucao", "eureka", "lightbulb",
    ]);

    // ⚠️ Atenção / Cuidado / Perigo / Alerta
    register(&mut map, "⚠️", &[
        "atencao", "cuidado", "perigo", "perigoso", "perigosa", "alerta", "pare", "stop",
        "erro", "erros", "proibido", "risco", "never", "nunca", "warning", "danger", "urgente",
    ]);

    // 🏆 Meta / Campeão / Troféu / Vitória
    register(&mut map, "🏆", &[
        "meta", "metas", "campeao", "campea", "trofeu", "vitoria", "vencer", "vencedor",
        "vencedora", "win", "winner", "trophy", "conquista", "conquistar", "ouro",
    ]);

    // ❤️ Amor / Paixão / Coração
    register(&mut map, "❤️", &[
        "amor", "paixao", "coracao", "amar", "love", "carinho", "gratidao", "apaixonado",
        "apaixonada", "heart",
    ]);

    // ⏱️ Tempo / Rápido / Hora / Relógio
    register(&mut map, "⏱️", &[
        "tempo", "rapido", "rapida", "hora", "horas", "relogio", "minuto", "minutos",
        "segundo", "segundos", "atrasado", "prazo", "deadline", "time", "fast", "timer",
        "cronometro", "agora", "pressa",
    ]);

    // 🚀 Foguete / Crescimento / Decolar
    register(&mut map, "🚀", &[
        "foguete", "rocket", "crescer", "crescimento", "escala", "escalar", "exponencial",
        "decolar", "decolou", "acelerar", "explosao",
    ]);

    // 🎯 Alvo / Foco / Estratégia
    register(&mut map, "🎯", &[
        "alvo", "foco", "focar", "focado", "focada", "target", "mira", "objetivo",
        "objetivos", "estrategia", "precisao",
    ]);

    // 🤯 Mente Explodindo / Choque
    register(&mut map, "🤯", &[
        "choque", "chocado", "chocada", "uau", "wow", "inacreditavel", "mindblowing",
    ]);

    // 😂 Riso / Humor / Meme
    register(&mut map, "😂", &[
        "engracado", "engracada", "rir", "riso", "risada", "piada", "meme", "zoeira",
        "humor", "kkk", "kkkk", "lol", "hilario",
    ]);

    // 💪 Força / Treino / Disciplina
    register(&mut map, "💪", &[
        "forca", "treino", "treinar", "academia", "monstro", "monstra", "disciplina",
        "gym", "forte", "musculo", "workout", "power",
    ]);

    // 👀 Olhos / Veja / Olha
    register(&mut map, "👀", &[
        "olha", "olhe", "olhem", "veja", "vejam", "observe", "repare", "look", "eyes",
    ]);

    // ✅ Check / Concluído / Correto
    register(&mut map, "✅", &[
        "certo", "certa", "check", "feito", "feita", "aprovado", "aprovada", "correto",
        "correta", "fechado", "fechada", "confirmado", "confirmada",
    ]);

    // ❌ Errado / Cancelado / Não
    register(&mut map, "❌", &[
        "errado", "errada", "falso", "falsa", "nao", "cancelado", "cancelada", "fail",
        "fracasso", "derrota", "banido",
    ]);

    // 🧠 Cérebro / Inteligência / Estudo
    register(&mut map, "🧠", &[
        "inteligencia", "inteligente", "cerebro", "estudar", "estudo", "aprender",
        "aprendizado", "sabedoria", "conhecimento", "brain",
    ]);

    // ✨ Magia / Transformação / Automação
    register(&mut map, "✨", &[
        "magica", "magico", "automacao", "automatizar", "ia", "ai", "futuro", "milagre",
        "especial", "brilho", "transformacao",
    ]);

    // 📈 Investimento / Gráfico de Alta
    register(&mut map, "📈", &[
        "investir", "investimento", "valorizacao", "alta", "evoluir", "rendimento",
    ]);

    // ❓ Pergunta / Dúvida
    register(&mut map, "❓", &[
        "duvida", "pergunta", "question", "porque", "qual", "quem",
    ]);

    map
});

fn register(map: &mut HashMap<&'static str, &'static str>, emoji: &'static str, words: &[&'static str]) {
    for &word in words {
        map.insert(word, emoji);
    }
}

pub struct EmojiDictionary;

impl EmojiDictionary {
    /// Normaliza a palavra removendo acentos, pontuações e caracteres especiais,
    /// convertendo para minúsculas.
    pub fn normalize(token: &str) -> String {
        if token.trim().is_empty() {
            return String::new();
        }

        token
            .chars()
            .filter(|c| c.is_alphanumeric())
            .map(|c| match c {
                'á' | 'à' | 'â' | 'ã' | 'ä' | 'Á' | 'À' | 'Â' | 'Ã' | 'Ä' => 'a',
                'é' | 'è' | 'ê' | 'ë' | 'É' | 'È' | 'Ê' | 'Ë' => 'e',
                'í' | 'ì' | 'î' | 'ï' | 'Í' | 'Ì' | 'Î' | 'Ï' => 'i',
                'ó' | 'ò' | 'ô' | 'õ' | 'ö' | 'Ó' | 'Ò' | 'Ô' | 'Õ' | 'Ö' => 'o',
                'ú' | 'ù' | 'û' | 'ü' | 'Ú' | 'Ù' | 'Û' | 'Ü' => 'u',
                'ç' | 'Ç' => 'c',
                'ñ' | 'Ñ' => 'n',
                other => other.to_ascii_lowercase(),
            })
            .collect::<String>()
            .trim()
            .to_string()
    }

    /// Busca um emoji viral semanticamente associado ao token fornecido.
    pub fn find_emoji(token: &str) -> Option<&'static str> {
        if token.trim().is_empty() {
            return None;
        }
        let normalized = Self::normalize(token);
        if normalized.is_empty() {
            return None;
        }
        DICTIONARY.get(normalized.as_str()).copied()
    }

    /// Verifica se o token fornecido possui um emoji associado no dicionário.
    pub fn has_emoji_match(token: &str) -> bool {
        Self::find_emoji(token).is_some()
    }
}

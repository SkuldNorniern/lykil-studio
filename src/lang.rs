//! English and Korean.
//!
//! Text is written in English in the code and looked up here; anything
//! missing from the table stays English. `{}` in a template is filled in
//! order by [`Lang::fill`].

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Lang {
    #[default]
    En,
    Ko,
}

impl Lang {
    /// `LYKIL_LANG` (`en` or `ko`) if set, else the system language.
    pub fn detect() -> Self {
        let wanted = std::env::var("LYKIL_LANG")
            .ok()
            .or_else(sys_locale::get_locale)
            .unwrap_or_default();
        if wanted.to_ascii_lowercase().starts_with("ko") {
            Self::Ko
        } else {
            Self::En
        }
    }

    pub const fn other(self) -> Self {
        match self {
            Self::En => Self::Ko,
            Self::Ko => Self::En,
        }
    }

    /// The name shown on the language switch.
    pub const fn label(self) -> &'static str {
        match self {
            Self::En => "English",
            Self::Ko => "한국어",
        }
    }

    /// A font with this language's letters.
    pub const fn font_family(self) -> &'static str {
        match self {
            Self::En => "Segoe UI",
            Self::Ko => "Malgun Gothic",
        }
    }

    pub fn tr(self, en: &'static str) -> &'static str {
        match self {
            Self::En => en,
            Self::Ko => KO.iter().find(|(e, _)| *e == en).map_or(en, |(_, ko)| *ko),
        }
    }

    /// [`Self::tr`], then each `{}` replaced by the next of `args`.
    pub fn fill(self, template: &'static str, args: &[&str]) -> String {
        let mut out = String::new();
        let mut rest = self.tr(template);
        for arg in args {
            let Some((head, tail)) = rest.split_once("{}") else {
                break;
            };
            out.push_str(head);
            out.push_str(arg);
            rest = tail;
        }
        out.push_str(rest);
        out
    }
}

const KO: &[(&str, &str)] = &[
    // header and footer
    ("Keymap", "키맵"),
    ("Macros", "매크로"),
    ("Lighting", "조명"),
    ("Device", "장치"),
    ("Connected", "연결됨"),
    ("Looking for a keyboard", "키보드를 찾는 중"),
    ("Connection lost", "연결 끊김"),
    ("No keyboard", "키보드 없음"),
    ("up {}   healthy", "가동 {}   정상"),
    ("up {}   {} faults", "가동 {}   오류 {}"),
    ("Plug in a Lykil keyboard", "Lykil 키보드를 연결하세요"),
    (
        "Studio finds it on its own. Close VIA if it is open.",
        "Studio가 알아서 찾습니다. VIA가 열려 있다면 닫아 주세요.",
    ),
    ("The connection was lost: {}", "연결이 끊겼습니다: {}"),
    (
        "the keyboard refused the change: {}",
        "키보드가 변경을 거부했습니다: {}",
    ),
    // keymap
    ("LAYER", "레이어"),
    ("Reset keymap", "키맵 초기화"),
    (
        "Click again to reset every layer",
        "한 번 더 누르면 모든 레이어를 초기화합니다",
    ),
    ("Click a key to change what it does", "바꿀 키를 누르세요"),
    (
        "Click a key, then pick a binding. Changes are saved on the keyboard.",
        "키를 누르고 동작을 고르세요. 바뀐 내용은 키보드에 저장됩니다.",
    ),
    ("{} on {}", "{} ({})"),
    (
        "Editing {} on {}. A pick moves on to the next key. Esc to stop.",
        "{} 편집 중 ({}). 고르면 다음 키로 넘어갑니다. Esc로 끝냅니다.",
    ),
    ("WHEN HELD", "누르고 있으면"),
    ("tap only", "탭만"),
    ("SEND WITH", "함께 보낼 키"),
    ("Letters", "문자"),
    ("Numbers and symbols", "숫자와 기호"),
    ("Editing", "편집"),
    ("Navigation", "이동"),
    ("Function", "기능 키"),
    ("Modifiers", "수정자 키"),
    ("Media", "미디어"),
    ("Keypad", "키패드"),
    ("Layers: hold", "레이어: 누르는 동안"),
    ("Layers: toggle", "레이어: 전환"),
    ("One-shot", "원샷"),
    ("Special", "특수"),
    // macros
    (
        "This keyboard's firmware has no macros yet",
        "이 키보드의 펌웨어는 아직 매크로가 없습니다",
    ),
    ("Macro {}", "매크로 {}"),
    ("empty", "비어 있음"),
    ("1 step", "1단계"),
    ("{} steps", "{}단계"),
    ("TYPES", "입력하는 글"),
    (
        "Start typing: this macro will type the same text.",
        "입력을 시작하세요. 이 매크로가 같은 글을 입력합니다.",
    ),
    ("{} / {} steps", "{} / {}단계"),
    ("Save to keyboard", "키보드에 저장"),
    ("Clear", "지우기"),
    (
        "Bind it on the keymap page: Macros group, {}. Typed as a US layout.",
        "키맵 화면의 매크로 묶음에서 {}을(를) 키에 지정하세요. 미국식 배열로 입력됩니다.",
    ),
    (
        "Typing into the macro. Save sends it to the keyboard; Esc throws it away.",
        "매크로를 입력하는 중입니다. 저장하면 키보드로 보내고, Esc를 누르면 버립니다.",
    ),
    (
        "Pick a macro and type. Letters, digits, symbols, space, Enter and Tab.",
        "매크로를 고르고 입력하세요. 영문자, 숫자, 기호, 스페이스, Enter, Tab을 쓸 수 있습니다.",
    ),
    (
        "too long: {} steps, a macro holds {}",
        "너무 깁니다: {}단계, 매크로는 {}단계까지입니다",
    ),
    // lighting
    (
        "This keyboard has no lighting",
        "이 키보드에는 조명이 없습니다",
    ),
    ("WHO CONTROLS THE LIGHTS", "조명 제어"),
    ("Keyboard effects", "키보드 효과"),
    ("Windows Dynamic Lighting", "Windows 동적 조명"),
    (
        "Windows may take the LEDs (Settings > Personalization > Dynamic Lighting). The effect runs while it does not.",
        "Windows가 LED를 제어할 수 있습니다 (설정 > 개인 설정 > 동적 조명). 그렇지 않을 때는 효과가 켜집니다.",
    ),
    (
        "The keyboard runs its own effect; Windows is ignored.",
        "키보드가 자체 효과를 켜고 Windows 요청은 무시합니다.",
    ),
    ("EFFECT", "효과"),
    ("Off", "끄기"),
    ("Solid", "단색"),
    ("Breathing", "숨쉬기"),
    ("Cycle", "색 순환"),
    ("Wave", "물결"),
    ("Reactive", "반응형"),
    ("Per-key", "키별 색"),
    ("Ripple", "파문"),
    ("LEDs off", "LED 끄기"),
    ("One steady colour", "한 가지 색을 계속"),
    ("Fades in and out", "밝아졌다 어두워졌다"),
    ("Round the colour wheel", "색상환을 따라 돌기"),
    ("A rainbow moving across", "옆으로 흐르는 무지개"),
    ("Dim; pressed keys flash", "은은하게, 누른 키는 번쩍"),
    ("Rings from pressed keys", "누른 키에서 퍼지는 고리"),
    ("Paint every key", "키마다 색 칠하기"),
    ("Paint all", "모두 칠하기"),
    ("Clear all", "모두 지우기"),
    ("COLOUR", "색"),
    ("BRUSH", "붓"),
    ("RECENT", "최근"),
    ("not a colour: {}", "색이 아닙니다: {}"),
    (
        "Click or drag to paint, right click takes a key's colour. Ctrl+C and Ctrl+V copy and paste colours.",
        "누르거나 끌어서 칠하고, 오른쪽 클릭으로 키의 색을 가져옵니다. Ctrl+C와 Ctrl+V로 색을 복사하고 붙여 넣습니다.",
    ),
    (
        "Type on the keyboard, or click keys here, to see it.",
        "키보드를 치거나 여기서 키를 눌러 확인하세요.",
    ),
    ("BRIGHTNESS", "밝기"),
    ("SPEED", "속도"),
    (
        "An app or Windows is setting the colours right now.",
        "지금은 앱이나 Windows가 색을 정하고 있습니다.",
    ),
    (
        "The LED driver chips do not answer; the keyboard keeps trying.",
        "LED 드라이버 칩이 응답하지 않습니다. 키보드가 계속 다시 시도합니다.",
    ),
    (
        "{} LEDs. Settings are saved on the keyboard.",
        "LED {}개. 설정은 키보드에 저장됩니다.",
    ),
    // device
    ("LAYOUT", "배열"),
    ("MATRIX", "매트릭스"),
    ("PROTOCOL", "프로토콜"),
    ("UPTIME", "가동 시간"),
    ("SCANS", "스캔"),
    ("KEY CHANGES", "키 변화"),
    ("FAULTS", "오류"),
    ("WATCHDOG RESETS", "워치독 재시작"),
    ("LAST START", "마지막 시작"),
    ("STORAGE", "저장소"),
    ("{} keys, {} layers", "키 {}개, 레이어 {}개"),
    ("{} ({} raw)", "{} (원시 {})"),
    ("ok", "정상"),
    ("failed", "실패"),
    ("none", "없음"),
    ("power", "전원"),
    ("pin", "리셋 버튼"),
    ("software", "소프트웨어"),
    ("watchdog", "워치독"),
    ("other", "기타"),
    ("unknown", "알 수 없음"),
    (
        "Keys light up while pressed: a quick way to check every switch.",
        "누르는 키가 표시됩니다. 스위치를 하나씩 확인하기 좋습니다.",
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_text_stays_english_and_templates_fill() {
        assert_eq!(Lang::Ko.tr("Keymap"), "키맵");
        assert_eq!(Lang::Ko.tr("not in the table"), "not in the table");
        assert_eq!(
            Lang::En.fill("{} keys, {} layers", &["87", "4"]),
            "87 keys, 4 layers"
        );
        assert_eq!(
            Lang::Ko.fill("{} keys, {} layers", &["87", "4"]),
            "키 87개, 레이어 4개"
        );
    }

    #[test]
    fn every_template_keeps_its_placeholders() {
        for (en, ko) in KO {
            assert_eq!(en.matches("{}").count(), ko.matches("{}").count(), "{en}");
        }
    }
}

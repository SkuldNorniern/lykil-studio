//! English and Korean. Missing text stays English.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Lang {
    #[default]
    En,
    Ko,
}

impl Lang {
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

    pub const fn label(self) -> &'static str {
        match self {
            Self::En => "English",
            Self::Ko => "한국어",
        }
    }

    /// Off Windows, Aurea finds fonts by file name.
    pub const fn font_family(self) -> &'static str {
        if cfg!(windows) {
            match self {
                Self::En => "Segoe UI",
                Self::Ko => "Malgun Gothic",
            }
        } else if cfg!(target_os = "macos") {
            match self {
                Self::En => "Helvetica",
                Self::Ko => "AppleSDGothicNeo",
            }
        } else {
            match self {
                Self::En => "DejaVuSans",
                Self::Ko => "NotoSansCJK",
            }
        }
    }

    pub fn tr(self, en: &'static str) -> &'static str {
        match self {
            Self::En => en,
            Self::Ko => KO.iter().find(|(e, _)| *e == en).map_or(en, |(_, ko)| *ko),
        }
    }

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
    (
        "Your Lykil keyboard is a Dynamic Lighting device: Windows can light it together with your other devices.",
        "Lykil 키보드는 동적 조명 장치입니다. Windows가 다른 장치와 함께 불을 켤 수 있습니다.",
    ),
    ("THIS KEYBOARD", "이 키보드"),
    ("Windows may take the LEDs", "Windows가 LED를 제어함"),
    (
        "The keyboard keeps its own effect",
        "키보드가 자체 효과를 유지함",
    ),
    ("Let Windows take the LEDs", "Windows에 LED 맡기기"),
    ("Keep the keyboard's effect", "키보드 효과 유지"),
    ("Open Dynamic Lighting settings", "동적 조명 설정 열기"),
    ("OTHER DEVICES", "다른 장치"),
    ("Leave them to Windows", "Windows에 맡기기"),
    (
        "Light them with the keyboard's effect",
        "키보드 효과로 켜기",
    ),
    (
        "No Dynamic Lighting devices found.",
        "동적 조명 장치를 찾지 못했습니다.",
    ),
    ("{}, {} lamps", "{}, 조명 {}개"),
    (
        "Windows has not handed it over yet",
        "Windows가 아직 넘겨주지 않았습니다",
    ),
    ("Follows the keyboard", "키보드를 따름"),
    ("Follow the keyboard", "키보드 따르기"),
    ("Following", "따르는 중"),
    (
        "Windows controls these lights now. Studio only shows where they sit.",
        "지금은 Windows가 이 조명을 제어합니다. Studio는 놓인 자리만 보여 줍니다.",
    ),
    ("Windows controls it", "Windows가 제어 중"),
    (
        "Not following: Windows controls it",
        "따르지 않음: Windows가 제어 중",
    ),
    (
        "Waiting for Windows to hand it over",
        "Windows가 넘겨주기를 기다리는 중",
    ),
    (
        "Waiting: Studio has to be the window in front",
        "대기 중: Studio 창이 앞에 있어야 합니다",
    ),
    ("Studio lights it", "Studio가 켜는 중"),
    (
        "The LED drivers do not answer, so the keyboard stays dark.",
        "LED 드라이버가 응답하지 않아 키보드가 꺼져 있습니다.",
    ),
    (
        "Windows Dynamic Lighting has the LEDs: this effect is paused.",
        "Windows 동적 조명이 LED를 쓰고 있어 이 효과는 멈춰 있습니다.",
    ),
    (
        "An app has the LEDs: this effect is paused.",
        "앱이 LED를 쓰고 있어 이 효과는 멈춰 있습니다.",
    ),
    (
        "The keyboard runs this effect. Windows may take the LEDs at any time.",
        "키보드가 이 효과를 켜고 있습니다. Windows가 언제든 LED를 가져갈 수 있습니다.",
    ),
    (
        "Studio lights {} other devices with it.",
        "Studio가 다른 장치 {}개도 함께 켭니다.",
    ),
    (
        "Other devices wait for Studio to be in front.",
        "다른 장치는 Studio 창이 앞에 오기를 기다립니다.",
    ),
    ("Used while Studio lights it.", "Studio가 켤 때 쓰입니다."),
    ("Follow", "따르기"),
    (
        "Drag the devices to where they sit",
        "장치를 실제 놓인 자리로 끌어 옮기세요",
    ),
    (
        "Click a device on the desk to set it up",
        "책상 위 장치를 눌러 설정하세요",
    ),
    ("keyboard", "키보드"),
    ("mouse", "마우스"),
    ("game controller", "게임 컨트롤러"),
    ("peripheral", "주변기기"),
    ("chassis", "본체"),
    ("headset", "헤드셋"),
    ("device", "장치"),
    (
        "Studio lights other devices only while it is the window in front; Windows keeps background control for packaged apps.",
        "Studio는 창이 앞에 있을 때만 다른 장치를 켭니다. 백그라운드 제어는 Windows가 패키지 앱에만 허용합니다.",
    ),
    (
        "Mice, cases and other lit devices are set in Windows Settings for now. Driving them from Studio, in step with the keyboard, comes later.",
        "마우스, 케이스 같은 다른 조명 장치는 지금은 Windows 설정에서 정합니다. Studio에서 키보드와 맞춰 켜는 기능은 나중에 들어옵니다.",
    ),
    (
        "Connect the keyboard to change how it shares its LEDs.",
        "LED를 어떻게 맡길지 바꾸려면 키보드를 연결하세요.",
    ),
    (
        "Ctrl+1 to Ctrl+5 or Ctrl+Tab switch pages",
        "Ctrl+1~5 또는 Ctrl+Tab으로 화면을 바꿉니다",
    ),
    ("Connected", "연결됨"),
    ("Connected over VIA", "VIA로 연결됨"),
    ("VIA definition needed", "VIA 정의 파일 필요"),
    ("{} speaks VIA", "{}은(는) VIA를 씁니다"),
    (
        "Studio needs its VIA definition, the JSON VIA's Design tab loads (vendor {}, product {}).",
        "Studio에는 VIA 정의 파일이 필요합니다. VIA의 Design 탭에서 여는 JSON입니다 (벤더 {}, 제품 {}).",
    ),
    (
        "Put the file in this folder; Studio picks it up by itself:",
        "이 폴더에 파일을 넣으면 Studio가 알아서 읽습니다:",
    ),
    ("Open the folder", "폴더 열기"),
    ("USB ID", "USB ID"),
    (
        "VIA keyboards have no lighting or macros in Studio",
        "VIA 키보드는 Studio에서 조명과 매크로를 쓸 수 없습니다",
    ),
    (
        "VIA has no keycode for this binding",
        "VIA에는 이 동작의 키코드가 없습니다",
    ),
    ("Looking for a keyboard", "키보드를 찾는 중"),
    ("Connection lost", "연결 끊김"),
    ("No keyboard", "키보드 없음"),
    ("up {}   healthy", "가동 {}   정상"),
    ("up {}   {} faults", "가동 {}   오류 {}"),
    (
        "Plug in a Lykil or VIA keyboard",
        "Lykil 또는 VIA 키보드를 연결하세요",
    ),
    ("Seen, but no answer:", "보이지만 응답이 없음:"),
    ("{} (Lykil only)", "{} (Lykil 전용)"),
    (
        "The keyboard runs this effect. The picture shows its colour, VIA effects are not played here.",
        "키보드가 이 효과를 켜고 있습니다. 그림은 색만 보여 주고, VIA 효과는 여기서 재생하지 않습니다.",
    ),
    (
        "This section has no effects to pick",
        "이 항목에는 고를 효과가 없습니다",
    ),
    (
        "This section has no colour setting",
        "이 항목에는 색 설정이 없습니다",
    ),
    (
        "The keyboard restarts in a second so Windows sees the change.",
        "Windows가 바뀐 설정을 알 수 있도록 키보드가 잠시 뒤 다시 시작합니다.",
    ),
    (
        "The keyboard runs this effect. Windows does not see it as a lighting device.",
        "키보드가 이 효과를 켜고 있습니다. Windows는 이 키보드를 조명 장치로 보지 않습니다.",
    ),
    ("The keyboard's", "키보드와 같이"),
    ("Its own", "따로"),
    (
        "This VIA keyboard has no dynamic macros",
        "이 VIA 키보드에는 동적 매크로가 없습니다",
    ),
    (
        "This keyboard's VIA definition has no lighting menu",
        "이 키보드의 VIA 정의에는 조명 메뉴가 없습니다",
    ),
    (
        "VIA settings from the keyboard's definition. Saved on the keyboard when you let go.",
        "키보드 정의에서 가져온 VIA 설정입니다. 손을 떼면 키보드에 저장됩니다.",
    ),
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
    (
        "Too long to save: {} steps over. Shorten the text.",
        "너무 길어 저장할 수 없습니다: {}단계 넘습니다. 글을 줄이세요.",
    ),
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
    ("Starlight", "별빛"),
    ("Rain", "비"),
    ("Heatmap", "열지도"),
    ("Keys twinkle at random", "키가 여기저기 반짝임"),
    ("Drops fall down the board", "물방울이 위에서 아래로"),
    ("Keys warm up as you type", "칠수록 키가 달아오름"),
    ("One colour", "한 가지 색"),
    ("Two colours", "두 가지 색"),
    ("Rainbow", "무지개"),
    ("Colour 1", "색 1"),
    ("Colour 2", "색 2"),
    ("BACKGROUND", "배경 밝기"),
    ("SIZE", "크기"),
    ("LIGHT THE KEYS OF A HELD LAYER", "누른 레이어의 키 표시"),
    ("On", "켜기"),
    ("LEDs off", "LED 끄기"),
    ("One steady colour", "한 가지 색을 계속"),
    ("Fades in and out", "밝아졌다 어두워졌다"),
    ("Pressed keys light up", "누른 키가 밝아짐"),
    ("Waves from pressed keys", "누른 키에서 물결이 퍼짐"),
    ("Keys change at random", "키가 무작위로 바뀜"),
    (
        "Steps through red, green and blue",
        "빨강, 초록, 파랑을 차례로",
    ),
    (
        "Letters and mods in two colours",
        "글자와 조합키를 두 색으로",
    ),
    ("Red and green", "빨강과 초록"),
    ("Colours moving across", "색이 가로질러 움직임"),
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
    (
        "Windows Dynamic Lighting has the LEDs, so the effect below does not run.",
        "Windows 동적 조명이 LED를 제어하고 있어서 아래 효과가 켜지지 않습니다.",
    ),
    (
        "An app has the LEDs; the effect comes back when it lets go.",
        "앱이 LED를 제어하고 있습니다. 앱이 놓으면 효과가 돌아옵니다.",
    ),
    ("Use keyboard effects", "키보드 효과 사용"),
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
    ("FIRMWARE", "펌웨어"),
    ("CHIP ID", "칩 ID"),
    ("update it to see", "업데이트하면 보입니다"),
    (
        "Arrow keys move to the next key, Delete clears it, Esc lets go.",
        "화살표로 다음 키로 가고, Delete로 비우고, Esc로 끝냅니다.",
    ),
    (
        "Pick a layer above; see-through keys use the layer below.",
        "위에서 레이어를 고르세요. 투명한 키는 아래 레이어를 씁니다.",
    ),
    (
        "When held makes a key do two things: tap for one, hold for another.",
        "누르고 있으면: 한 키로 두 가지. 톡 치면 하나, 누르고 있으면 다른 하나.",
    ),
    (
        "Send with adds modifiers, so one key can type Shift+1 or Ctrl+C.",
        "함께 보낼 키: 수정자 키를 더해 한 키로 Shift+1이나 Ctrl+C를 보냅니다.",
    ),
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
    fn each_text_is_in_the_table_once() {
        let mut seen = std::collections::BTreeSet::new();
        let twice: Vec<&str> = KO
            .iter()
            .map(|(en, _)| *en)
            .filter(|en| !seen.insert(*en))
            .collect();
        assert!(twice.is_empty(), "twice in KO: {twice:?}");
    }

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

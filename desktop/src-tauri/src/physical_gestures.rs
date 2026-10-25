//! Physical tap gestures. No key text, clock, native handles or session side effects.
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Idle,
    Recording,
    Busy,
    Suspended,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Intent {
    Dictation,
    Translation,
    Ask,
    Stop,
    Cancel,
}
pub struct KeyDecision {
    pub intent: Option<Intent>,
    pub suppress: bool,
}

#[derive(Clone, Debug)]
pub struct Bindings {
    pub dictation: BTreeSet<u16>,
    pub translation: BTreeSet<u16>,
    pub ask: BTreeSet<u16>,
    pub stop: BTreeSet<u16>,
}
impl Default for Bindings {
    fn default() -> Self {
        Self {
            dictation: [0xa5].into(),
            translation: [0xa5, 0xa1].into(),
            ask: [0xa5, 0x20].into(),
            stop: [0xa5].into(),
        }
    }
}
impl Bindings {
    pub fn parse(
        dictation: &str,
        translation: &str,
        ask: &str,
        stop: &str,
    ) -> Result<Self, String> {
        let result = Self {
            dictation: parse_chord(dictation)?,
            translation: parse_chord(translation)?,
            ask: parse_chord(ask)?,
            stop: parse_chord(stop)?,
        };
        if result.dictation == result.translation
            || result.dictation == result.ask
            || result.translation == result.ask
        {
            return Err("shortcut-start-conflict".into());
        }
        Ok(result)
    }
}
pub fn parse_chord(text: &str) -> Result<BTreeSet<u16>, String> {
    let mut keys = BTreeSet::new();
    for token in text.trim().to_ascii_lowercase().split('+') {
        let key = match token.trim() {
            "alt_right" | "ralt" => 0xa5,
            "alt_left" | "lalt" => 0xa4,
            "shift_right" | "rshift" => 0xa1,
            "shift_left" | "lshift" => 0xa0,
            "ctrl_right" | "rctrl" => 0xa3,
            "ctrl_left" | "lctrl" => 0xa2,
            "super_left" => 0x5b,
            "super_right" => 0x5c,
            "space" => 0x20,
            t if t.starts_with('f') && t.len() > 1 => {
                let number = t[1..].parse::<u16>().map_err(|_| "shortcut-invalid")?;
                if !(1..=24).contains(&number) {
                    return Err("shortcut-invalid".into());
                }
                0x6f + number
            }
            t if t.len() == 1 && t.as_bytes()[0].is_ascii_alphanumeric() => {
                t.as_bytes()[0].to_ascii_uppercase() as u16
            }
            _ => return Err("shortcut-physical-key-required".into()),
        };
        if !keys.insert(key) {
            return Err("shortcut-invalid".into());
        }
    }
    let regular: Vec<_> = keys
        .iter()
        .filter(|&&k| !(0xa0..=0xa5).contains(&k) && k != 0x5b && k != 0x5c)
        .collect();
    if keys.is_empty()
        || regular.len() > 1
        || (keys.len() == 1
            && regular
                .first()
                .is_some_and(|&&k| !(0x70..=0x87).contains(&k)))
    {
        return Err("shortcut-invalid".into());
    }
    Ok(keys)
}

pub struct GestureRouter {
    bindings: Bindings,
    held: BTreeSet<u16>,
    seen: BTreeSet<u16>,
    phase: Option<Phase>,
    invalid: bool,
    suppressed: BTreeSet<u16>,
}
impl GestureRouter {
    pub fn new(bindings: Bindings) -> Self {
        Self {
            bindings,
            held: BTreeSet::new(),
            seen: BTreeSet::new(),
            phase: None,
            invalid: false,
            suppressed: BTreeSet::new(),
        }
    }
    /// Input loss/settings changes invalidate a partial gesture; its releases drain.
    pub fn reset(&mut self) {
        self.invalid = true;
        self.seen.clear();
        self.phase = None;
    }
    pub fn configure(&mut self, bindings: Bindings) {
        self.reset();
        self.bindings = bindings;
    }
    pub fn held(&self) -> impl Iterator<Item = u16> + '_ {
        self.held.iter().copied()
    }
    /// Decides both the lifecycle intent and paired OS forwarding at one public seam.
    pub fn event(&mut self, key: u16, down: bool, phase: Phase) -> KeyDecision {
        let bindings: Vec<_> = if phase == Phase::Suspended {
            vec![]
        } else if phase == Phase::Recording {
            vec![
                &self.bindings.dictation,
                &self.bindings.translation,
                &self.bindings.ask,
                &self.bindings.stop,
            ]
        } else {
            vec![
                &self.bindings.dictation,
                &self.bindings.translation,
                &self.bindings.ask,
            ]
        };
        let standalone = bindings
            .iter()
            .any(|binding| binding.len() == 1 && binding.contains(&key));
        let anchor = self
            .held
            .iter()
            .any(|key| bindings.iter().any(|b| b.len() == 1 && b.contains(key)));
        let mut chord = self.held.clone();
        chord.insert(key);
        let modifier = (0xa0..=0xa5).contains(&key) || key == 0x5b || key == 0x5c;
        let complete = !modifier
            && bindings
                .iter()
                .any(|b| b.contains(&key) && b.is_subset(&chord));
        let reserved_suffix = anchor && bindings.iter().any(|binding| binding.contains(&key));
        let suppress = if down {
            let suppress = standalone
                || reserved_suffix
                || complete
                || (key == 0x1b && matches!(phase, Phase::Recording | Phase::Busy));
            if suppress {
                self.suppressed.insert(key);
            }
            suppress
        } else {
            self.suppressed.remove(&key)
        };
        let intent = self.edge(key, down, phase);
        KeyDecision { intent, suppress }
    }
    pub fn edge(&mut self, key: u16, down: bool, phase: Phase) -> Option<Intent> {
        if down {
            if self.held.is_empty() {
                self.phase = Some(phase);
                self.invalid = matches!(phase, Phase::Busy | Phase::Suspended);
            }
            if !self.held.insert(key) {
                return None;
            }
            self.seen.insert(key);
            if key == 0x1b {
                self.invalid = true;
                return matches!(phase, Phase::Recording | Phase::Busy).then_some(Intent::Cancel);
            }
            return None;
        }
        if !self.held.remove(&key) || !self.held.is_empty() {
            return None;
        }
        let seen = std::mem::take(&mut self.seen);
        let initial = self.phase.take();
        if std::mem::take(&mut self.invalid) || initial != Some(phase) {
            return None;
        }
        match phase {
            Phase::Idle if seen == self.bindings.dictation => Some(Intent::Dictation),
            Phase::Idle if seen == self.bindings.translation => Some(Intent::Translation),
            Phase::Idle if seen == self.bindings.ask => Some(Intent::Ask),
            Phase::Recording if seen == self.bindings.stop => Some(Intent::Stop),
            _ => None,
        }
    }
}

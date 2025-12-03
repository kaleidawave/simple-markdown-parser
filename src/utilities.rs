pub(crate) fn strip_number_prefix(on: &str) -> Option<&str> {
    let level = on.chars().take_while(|c| matches!(*c, '1'..'9')).count();
    if 0 < level && level < 9 {
        on[level..].strip_prefix([')', '.'])
    } else {
        None
    }
}

pub(crate) fn strip_surrounds<'a>(on: &'a str, left: &str, right: &str) -> Option<&'a str> {
    on.trim()
        .strip_prefix(left)
        .and_then(|line| line.strip_suffix(right))
        .map(str::trim)
}

pub(crate) fn strip_upto_three_spaces(on: &str) -> &str {
    let level = on.find(|c| c != ' ').unwrap_or(on.len());
    if level <= 3 {
        &on[level..]
    } else {
        on
    }
}

pub(crate) fn starts_with_new_line_sequence(on: &str) -> bool {
    on.starts_with("\r\n") || on.starts_with('\n')
}

pub(crate) fn count_new_line_sequence(on: &str) -> usize {
    if on.starts_with("\r\n") {
        2
    } else if on.starts_with('\n') {
        1
    } else {
        panic!("string does not start with new line sequence")
    }
}

pub(crate) fn find_new_line_sequence(on: &str) -> Option<(usize, usize)> {
    if let Some((idx, matched)) = on.match_indices(['\r', '\n']).next() {
        // TODO does this check need to be done?
        let len = if matched == "\r" && on[idx..].starts_with("\r\n") {
            2
        } else {
            1
        };
        Some((idx, len))
    } else {
        None
    }
}

pub struct EdibleLines<'a> {
    start: usize,
    last: usize,
    on: &'a str,
}

impl<'a> EdibleLines<'a> {
    #[must_use]
    pub fn new(on: &'a str) -> Self {
        EdibleLines {
            on,
            start: 0,
            last: 0,
        }
    }

    pub fn moving_on(&mut self) {
        self.start = self.last;
    }
    #[must_use]
    pub fn peek_line(&self) -> Option<&'a str> {
        self.on
            .get(self.last..)
            .map(|rest| rest.lines().next().unwrap_or(rest))
    }
    #[must_use]
    pub fn on(&self) -> &'a str {
        self.on
    }
    #[must_use]
    pub fn is_at_start(&self) -> bool {
        self.start == 0
    }

    pub fn skip_next(&mut self) {
        if let Some((next, len)) = find_new_line_sequence(&self.on[self.last..]) {
            self.last += next + len;
        } else {
            self.last = self.on.len();
        }
    }
}

impl<'a> Iterator for EdibleLines<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((next, len)) = find_new_line_sequence(&self.on[self.last..]) {
            let slice = &self.on[self.start..(self.last + next)];
            self.last += next + len;
            Some(slice)
        } else if self.last < self.on.len() {
            self.last = self.on.len();
            Some(&self.on[self.start..])
        } else {
            None
        }
    }
}

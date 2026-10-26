//! A small LaTeX → Typst math translator for the formulas people usually write
//! in Markdown. Anything it does not understand returns `None`, and the
//! exporter falls back to showing the TeX source.

const GREEK: &[&str] = &[
    "alpha",
    "beta",
    "gamma",
    "delta",
    "epsilon",
    "zeta",
    "eta",
    "theta",
    "iota",
    "kappa",
    "lambda",
    "mu",
    "nu",
    "xi",
    "omicron",
    "pi",
    "rho",
    "sigma",
    "tau",
    "upsilon",
    "phi",
    "chi",
    "psi",
    "omega",
    "Gamma",
    "Delta",
    "Theta",
    "Lambda",
    "Xi",
    "Pi",
    "Sigma",
    "Upsilon",
    "Phi",
    "Psi",
    "Omega",
    "varepsilon",
    "varphi",
    "vartheta",
];

fn symbol(cmd: &str) -> Option<&'static str> {
    Some(match cmd {
        "infty" => "infinity",
        "cdot" => "dot.c",
        "cdots" => "dots.c",
        "ldots" | "dots" => "dots",
        "times" => "times",
        "div" => "div",
        "pm" => "plus.minus",
        "mp" => "minus.plus",
        "leq" | "le" => "<=",
        "geq" | "ge" => ">=",
        "neq" | "ne" => "!=",
        "approx" => "approx",
        "equiv" => "equiv",
        "sim" => "tilde.op",
        "propto" => "prop",
        "to" | "rightarrow" => "->",
        "leftarrow" => "<-",
        "Rightarrow" | "implies" => "=>",
        "Leftrightarrow" | "iff" => "<=>",
        "in" => "in",
        "notin" => "in.not",
        "subset" => "subset",
        "subseteq" => "subset.eq",
        "cup" => "union",
        "cap" => "sect",
        "forall" => "forall",
        "exists" => "exists",
        "partial" => "diff",
        "nabla" => "nabla",
        "sum" => "sum",
        "prod" => "product",
        "int" => "integral",
        "oint" => "integral.cont",
        "lim" => "lim",
        "log" => "log",
        "ln" => "ln",
        "exp" => "exp",
        "sin" => "sin",
        "cos" => "cos",
        "tan" => "tan",
        "max" => "max",
        "min" => "min",
        "quad" => "quad",
        "qquad" => "wide",
        "," | ";" | ":" | "!" => " ",
        "{" => "{",
        "}" => "}",
        "%" => "%",
        _ => return None,
    })
}

struct P<'a> {
    s: &'a [u8],
    i: usize,
}

impl P<'_> {
    fn peek(&self) -> Option<u8> {
        self.s.get(self.i).copied()
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\n' | b'\t' | b'\r')) {
            self.i += 1;
        }
    }

    /// One argument: `{...}` or a single token.
    fn arg(&mut self) -> Option<String> {
        self.skip_ws();
        let value = match self.peek()? {
            b'{' => {
                self.i += 1;
                let inner = self.seq(Some(b'}'))?;
                self.i += 1;
                Some(inner)
            }
            b'\\' => self.command(),
            _ => self.atom(),
        }?;
        Some(value.trim().to_string())
    }

    fn command(&mut self) -> Option<String> {
        self.i += 1; // backslash
        let start = self.i;
        while matches!(self.peek(), Some(c) if c.is_ascii_alphabetic()) {
            self.i += 1;
        }
        if self.i == start {
            // \, \; \{ and friends
            self.i += 1;
        }
        let name = std::str::from_utf8(&self.s[start..self.i]).ok()?;
        match name {
            "frac" | "dfrac" | "tfrac" => {
                let a = self.arg()?;
                let b = self.arg()?;
                Some(format!("frac({a}, {b})"))
            }
            "sqrt" => {
                self.skip_ws();
                if self.peek() == Some(b'[') {
                    self.i += 1;
                    let n = self.seq(Some(b']'))?;
                    self.i += 1;
                    let x = self.arg()?;
                    Some(format!("root({n}, {x})"))
                } else {
                    Some(format!("sqrt({})", self.arg()?))
                }
            }
            "text" | "mathrm" | "textrm" | "operatorname" => {
                self.skip_ws();
                if self.peek() != Some(b'{') {
                    return None;
                }
                self.i += 1;
                let start = self.i;
                while self.peek()? != b'}' {
                    self.i += 1;
                }
                let text = std::str::from_utf8(&self.s[start..self.i])
                    .ok()?
                    .replace('"', "\\\"");
                self.i += 1;
                Some(format!("upright(\"{text}\")"))
            }
            "mathbf" | "boldsymbol" => Some(format!("bold({})", self.arg()?)),
            "mathit" => Some(format!("italic({})", self.arg()?)),
            "mathbb" => Some(format!("bb({})", self.arg()?)),
            "mathcal" => Some(format!("cal({})", self.arg()?)),
            "hat" => Some(format!("hat({})", self.arg()?)),
            "bar" | "overline" => Some(format!("overline({})", self.arg()?)),
            "vec" => Some(format!("arrow({})", self.arg()?)),
            "left" | "right" | "big" | "Big" | "displaystyle" => Some(String::new()),
            _ if GREEK.contains(&name) => Some(format!("{name} ")),
            _ => symbol(name).map(|s| format!("{s} ")),
        }
    }

    fn atom(&mut self) -> Option<String> {
        let c = self.peek()?;
        self.i += 1;
        Some(match c {
            b'a'..=b'z' | b'A'..=b'Z' => format!("{} ", c as char),
            b'0'..=b'9' => {
                let start = self.i - 1;
                while matches!(self.peek(), Some(b'0'..=b'9' | b'.')) {
                    self.i += 1;
                }
                std::str::from_utf8(&self.s[start..self.i])
                    .ok()?
                    .to_string()
            }
            b'+' | b'-' | b'=' | b'<' | b'>' | b'(' | b')' | b'[' | b']' | b',' | b'|' | b'!'
            | b'\'' | b'*' => (c as char).to_string(),
            b'/' => "\\/".to_string(),
            b' ' | b'\n' | b'\t' | b'\r' => " ".to_string(),
            b'&' => return None,
            _ if c >= 0x80 => {
                // Pass UTF-8 sequences (e.g. ∞) through untouched.
                let start = self.i - 1;
                while matches!(self.peek(), Some(b) if (0x80..0xC0).contains(&b)) {
                    self.i += 1;
                }
                std::str::from_utf8(&self.s[start..self.i])
                    .ok()?
                    .to_string()
            }
            _ => return None,
        })
    }

    fn seq(&mut self, until: Option<u8>) -> Option<String> {
        let mut out = String::new();
        loop {
            match self.peek() {
                None => return if until.is_none() { Some(out) } else { None },
                Some(c) if Some(c) == until => return Some(out),
                Some(b'}') => return None,
                Some(b'^') | Some(b'_') => {
                    let op = self.peek()? as char;
                    self.i += 1;
                    let a = self.arg()?;
                    out = out.trim_end().to_string();
                    out.push(op);
                    out.push('(');
                    out.push_str(a.trim());
                    out.push(')');
                }
                Some(b'{') => {
                    self.i += 1;
                    let inner = self.seq(Some(b'}'))?;
                    self.i += 1;
                    out.push_str(&inner);
                }
                Some(b'\\') => {
                    if self.s.get(self.i + 1) == Some(&b'\\') {
                        out.push_str(" \\ ");
                        self.i += 2;
                    } else {
                        out.push_str(&self.command()?);
                    }
                }
                Some(_) => out.push_str(&self.atom()?),
            }
        }
    }
}

/// Translates TeX math to Typst math markup (without the `$` delimiters).
pub fn tex_to_typst(tex: &str) -> Option<String> {
    if tex.contains("\\begin") || tex.len() > 2000 {
        return None;
    }
    let mut p = P {
        s: tex.as_bytes(),
        i: 0,
    };
    let out = p.seq(None)?;
    let out = out.split_whitespace().collect::<Vec<_>>().join(" ");
    (!out.is_empty()).then_some(out)
}

#[cfg(test)]
mod tests {
    use super::tex_to_typst;

    #[test]
    fn translates_common_formulas() {
        assert_eq!(tex_to_typst("E = mc^2").as_deref(), Some("E = m c^(2)"));
        assert_eq!(
            tex_to_typst("a^2 + b^2 = c^2").as_deref(),
            Some("a^(2) + b^(2) = c^(2)")
        );
        assert_eq!(
            tex_to_typst(r"\frac{1}{2}\alpha").as_deref(),
            Some("frac(1, 2)alpha")
        );
        assert_eq!(
            tex_to_typst(r"\sum_{i=1}^{n} x_i").as_deref(),
            Some("sum_(i =1)^(n) x_(i)")
        );
        assert_eq!(
            tex_to_typst(r"\sqrt{x} \leq \infty").as_deref(),
            Some("sqrt(x) <= infinity")
        );
        assert_eq!(
            tex_to_typst(r"\text{area} = \pi r^2").as_deref(),
            Some("upright(\"area\") = pi r^(2)")
        );
    }

    #[test]
    fn gives_up_on_unknown_input() {
        assert_eq!(tex_to_typst(r"\begin{matrix} a \end{matrix}"), None);
        assert_eq!(tex_to_typst(r"\unknowncommand x"), None);
        assert_eq!(tex_to_typst("a & b"), None);
        assert_eq!(tex_to_typst("{unclosed"), None);
    }
}

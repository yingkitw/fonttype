/// Type 2 CharString decoder — converts CFF CharString bytecode into outline commands.

#[derive(Debug, Clone, PartialEq)]
pub enum PathCommand {
    MoveTo { x: f64, y: f64 },
    LineTo { x: f64, y: f64 },
    CurveTo { c1x: f64, c1y: f64, c2x: f64, c2y: f64, x: f64, y: f64 },
    ClosePath,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GlyphOutline {
    pub commands: Vec<PathCommand>,
    pub width: Option<f64>,
}

pub fn decode_charstring(data: &[u8], global_subrs: &[Vec<u8>], local_subrs: &[Vec<u8>]) -> Result<GlyphOutline, String> {
    let mut state = CharStringState::new(global_subrs, local_subrs);
    decode_stream(data, &mut state, 0)?;
    Ok(GlyphOutline {
        commands: state.commands,
        width: state.width,
    })
}

const MAX_DEPTH: usize = 10;

fn decode_stream(data: &[u8], state: &mut CharStringState, depth: usize) -> Result<(), String> {
    if depth > MAX_DEPTH {
        return Err("Subr call nesting too deep".to_string());
    }

    let mut i = 0;
    while i < data.len() {
        let b0 = data[i];

        // Type 2 CharString operators: 1-12, 14, 18-27, 29-31
        // 28 = shortint, 30 = real number (special number encodings)
        let is_operator = (b0 >= 1 && b0 <= 12)
            || b0 == 14
            || (b0 >= 18 && b0 <= 27)
            || b0 == 29
            || b0 == 30
            || b0 == 31;

        if is_operator {
            // Operator
            i += 1;
            let op = if b0 == 12 {
                if i >= data.len() {
                    return Err("Unexpected end of CharString after escape".to_string());
                }
                let b1 = data[i];
                i += 1;
                1200 + b1 as u16
            } else {
                b0 as u16
            };

            execute_operator(op, state, data, &mut i, depth)?;
        } else if b0 == 28 {
            // shortint: 2-byte signed
            if i + 2 >= data.len() {
                return Err("Unexpected end of CharString for shortint".to_string());
            }
            let val = i16::from_be_bytes([data[i + 1], data[i + 2]]) as i32;
            state.stack.push(val as f64);
            i += 3;
        } else if b0 == 29 {
            // 5-byte integer
            if i + 4 >= data.len() {
                return Err("Unexpected end of CharString for longint".to_string());
            }
            let val = i32::from_be_bytes([data[i + 1], data[i + 2], data[i + 3], data[i + 4]]);
            state.stack.push(val as f64);
            i += 5;
        } else if b0 == 30 {
            // real number — skip for basic decoding
            i += 1;
            while i < data.len() && data[i] & 0x0F != 0x0F {
                i += 1;
            }
            i += 1;
            // Push a placeholder; real numbers in charstrings are rare for outlines
            state.stack.push(0.0);
        } else if b0 >= 32 && b0 <= 246 {
            state.stack.push((b0 as i32 - 139) as f64);
            i += 1;
        } else if b0 >= 247 && b0 <= 250 {
            if i + 1 >= data.len() {
                return Err("Unexpected end of CharString".to_string());
            }
            let val = ((b0 as i32 - 247) * 256) + data[i + 1] as i32 + 108;
            state.stack.push(val as f64);
            i += 2;
        } else if b0 >= 251 && b0 <= 254 {
            if i + 1 >= data.len() {
                return Err("Unexpected end of CharString".to_string());
            }
            let val = -((b0 as i32 - 251) * 256) - data[i + 1] as i32 - 108;
            state.stack.push(val as f64);
            i += 2;
        } else {
            // b0 == 255 — reserved in Type 2, skip
            i += 1;
        }
    }

    Ok(())
}

struct CharStringState<'a> {
    stack: Vec<f64>,
    commands: Vec<PathCommand>,
    width: Option<f64>,
    x: f64,
    y: f64,
    global_subrs: &'a [Vec<u8>],
    local_subrs: &'a [Vec<u8>],
    has_width: bool,
    hint_count: usize,
}

impl<'a> CharStringState<'a> {
    fn new(global_subrs: &'a [Vec<u8>], local_subrs: &'a [Vec<u8>]) -> Self {
        CharStringState {
            stack: Vec::new(),
            commands: Vec::new(),
            width: None,
            x: 0.0,
            y: 0.0,
            global_subrs,
            local_subrs,
            has_width: false,
            hint_count: 0,
        }
    }

    fn maybe_extract_width(&mut self, expected_args: usize) {
        if !self.has_width && self.stack.len() == expected_args + 1 {
            self.width = Some(self.stack.remove(0));
            self.has_width = true;
        }
    }

    fn clear_stack(&mut self) {
        self.stack.clear();
    }
}

fn execute_operator(op: u16, state: &mut CharStringState, _data: &[u8], _i: &mut usize, depth: usize) -> Result<(), String> {
    match op {
        1 | 3 | 18 | 23 => {
            // hstem, vstem, hstemhm, vstemhm — hint operators, consume stack
            if !state.has_width && state.stack.len() % 2 == 1 {
                state.width = Some(state.stack.remove(0));
                state.has_width = true;
            }
            state.hint_count += state.stack.len() / 2;
            state.clear_stack();
        }
        4 => {
            // vmoveto
            state.maybe_extract_width(1);
            if state.stack.len() >= 1 {
                let dy = state.stack.pop().unwrap();
                state.y += dy;
                state.commands.push(PathCommand::MoveTo { x: state.x, y: state.y });
            }
            state.clear_stack();
        }
        5 => {
            // rlineto
            while state.stack.len() >= 2 {
                let dx = state.stack.remove(0);
                let dy = state.stack.remove(0);
                state.x += dx;
                state.y += dy;
                state.commands.push(PathCommand::LineTo { x: state.x, y: state.y });
            }
            state.clear_stack();
        }
        6 => {
            // hlineto
            let mut horizontal = true;
            while !state.stack.is_empty() {
                let d = state.stack.remove(0);
                if horizontal {
                    state.x += d;
                } else {
                    state.y += d;
                }
                state.commands.push(PathCommand::LineTo { x: state.x, y: state.y });
                horizontal = !horizontal;
            }
        }
        7 => {
            // vlineto
            let mut vertical = true;
            while !state.stack.is_empty() {
                let d = state.stack.remove(0);
                if vertical {
                    state.y += d;
                } else {
                    state.x += d;
                }
                state.commands.push(PathCommand::LineTo { x: state.x, y: state.y });
                vertical = !vertical;
            }
        }
        8 => {
            // rrcurveto
            while state.stack.len() >= 6 {
                let dx1 = state.stack.remove(0);
                let dy1 = state.stack.remove(0);
                let dx2 = state.stack.remove(0);
                let dy2 = state.stack.remove(0);
                let dx3 = state.stack.remove(0);
                let dy3 = state.stack.remove(0);
                let c1x = state.x + dx1;
                let c1y = state.y + dy1;
                let c2x = c1x + dx2;
                let c2y = c1y + dy2;
                state.x = c2x + dx3;
                state.y = c2y + dy3;
                state.commands.push(PathCommand::CurveTo {
                    c1x, c1y, c2x, c2y, x: state.x, y: state.y,
                });
            }
            state.clear_stack();
        }
        14 => {
            // endchar
            if !state.has_width && state.stack.len() == 1 {
                state.width = Some(state.stack.remove(0));
                state.has_width = true;
            }
            state.clear_stack();
        }
        21 => {
            // rmoveto
            state.maybe_extract_width(2);
            if state.stack.len() >= 2 {
                let dx = state.stack.remove(0);
                let dy = state.stack.remove(0);
                state.x += dx;
                state.y += dy;
                state.commands.push(PathCommand::MoveTo { x: state.x, y: state.y });
            }
            state.clear_stack();
        }
        22 => {
            // hmoveto
            state.maybe_extract_width(1);
            if state.stack.len() >= 1 {
                let dx = state.stack.pop().unwrap();
                state.x += dx;
                state.commands.push(PathCommand::MoveTo { x: state.x, y: state.y });
            }
            state.clear_stack();
        }
        24 => {
            // rcurveline
            while state.stack.len() >= 8 {
                let dx1 = state.stack.remove(0);
                let dy1 = state.stack.remove(0);
                let dx2 = state.stack.remove(0);
                let dy2 = state.stack.remove(0);
                let dx3 = state.stack.remove(0);
                let dy3 = state.stack.remove(0);
                let c1x = state.x + dx1;
                let c1y = state.y + dy1;
                let c2x = c1x + dx2;
                let c2y = c1y + dy2;
                state.x = c2x + dx3;
                state.y = c2y + dy3;
                state.commands.push(PathCommand::CurveTo {
                    c1x, c1y, c2x, c2y, x: state.x, y: state.y,
                });
            }
            if state.stack.len() >= 2 {
                let dx = state.stack.remove(0);
                let dy = state.stack.remove(0);
                state.x += dx;
                state.y += dy;
                state.commands.push(PathCommand::LineTo { x: state.x, y: state.y });
            }
            state.clear_stack();
        }
        25 => {
            // rlinecurve
            while state.stack.len() >= 8 {
                let dx1 = state.stack.remove(0);
                let dy1 = state.stack.remove(0);
                state.x += dx1;
                state.y += dy1;
                state.commands.push(PathCommand::LineTo { x: state.x, y: state.y });
                let dx2 = state.stack.remove(0);
                let dy2 = state.stack.remove(0);
                state.x += dx2;
                state.y += dy2;
                state.commands.push(PathCommand::LineTo { x: state.x, y: state.y });
            }
            if state.stack.len() >= 6 {
                let dx1 = state.stack.remove(0);
                let dy1 = state.stack.remove(0);
                let dx2 = state.stack.remove(0);
                let dy2 = state.stack.remove(0);
                let dx3 = state.stack.remove(0);
                let dy3 = state.stack.remove(0);
                let c1x = state.x + dx1;
                let c1y = state.y + dy1;
                let c2x = c1x + dx2;
                let c2y = c1y + dy2;
                state.x = c2x + dx3;
                state.y = c2y + dy3;
                state.commands.push(PathCommand::CurveTo {
                    c1x, c1y, c2x, c2y, x: state.x, y: state.y,
                });
            }
            state.clear_stack();
        }
        27 => {
            // hhcurveto
            if state.stack.len() >= 4 {
                if state.stack.len() % 4 == 1 {
                    let dy1 = state.stack.remove(0);
                    let dx1 = state.stack.remove(0);
                    let dx2 = state.stack.remove(0);
                    let dy2 = state.stack.remove(0);
                    let dx3 = state.stack.remove(0);
                    let c1x = state.x + dx1;
                    let c1y = state.y + dy1;
                    let c2x = c1x + dx2;
                    let c2y = c1y + dy2;
                    state.x = c2x + dx3;
                    state.commands.push(PathCommand::CurveTo {
                        c1x, c1y, c2x, c2y, x: state.x, y: state.y,
                    });
                }
                while state.stack.len() >= 4 {
                    let dx1 = state.stack.remove(0);
                    let dx2 = state.stack.remove(0);
                    let dy2 = state.stack.remove(0);
                    let dx3 = state.stack.remove(0);
                    let c1x = state.x + dx1;
                    let c1y = state.y;
                    let c2x = c1x + dx2;
                    let c2y = c1y + dy2;
                    state.x = c2x + dx3;
                    state.commands.push(PathCommand::CurveTo {
                        c1x, c1y, c2x, c2y, x: state.x, y: state.y,
                    });
                }
            }
            state.clear_stack();
        }
        30 => {
            // vhcurveto
            let mut vertical = true;
            while state.stack.len() >= 4 {
                if vertical {
                    let dy1 = state.stack.remove(0);
                    let dx2 = state.stack.remove(0);
                    let dy2 = state.stack.remove(0);
                    let dy3 = state.stack.remove(0);
                    let c1x = state.x;
                    let c1y = state.y + dy1;
                    let c2x = c1x + dx2;
                    let c2y = c1y + dy2;
                    state.y = c2y + dy3;
                    state.commands.push(PathCommand::CurveTo {
                        c1x, c1y, c2x, c2y, x: state.x, y: state.y,
                    });
                } else {
                    let dx1 = state.stack.remove(0);
                    let dx2 = state.stack.remove(0);
                    let dy2 = state.stack.remove(0);
                    let dx3 = state.stack.remove(0);
                    let c1x = state.x + dx1;
                    let c1y = state.y;
                    let c2x = c1x + dx2;
                    let c2y = c1y + dy2;
                    state.x = c2x + dx3;
                    state.commands.push(PathCommand::CurveTo {
                        c1x, c1y, c2x, c2y, x: state.x, y: state.y,
                    });
                }
                vertical = !vertical;
            }
            state.clear_stack();
        }
        31 => {
            // hvcurveto
            let mut horizontal = true;
            while state.stack.len() >= 4 {
                if horizontal {
                    let dx1 = state.stack.remove(0);
                    let dx2 = state.stack.remove(0);
                    let dy2 = state.stack.remove(0);
                    let dx3 = state.stack.remove(0);
                    let c1x = state.x + dx1;
                    let c1y = state.y;
                    let c2x = c1x + dx2;
                    let c2y = c1y + dy2;
                    state.x = c2x + dx3;
                    state.commands.push(PathCommand::CurveTo {
                        c1x, c1y, c2x, c2y, x: state.x, y: state.y,
                    });
                } else {
                    let dy1 = state.stack.remove(0);
                    let dx2 = state.stack.remove(0);
                    let dy2 = state.stack.remove(0);
                    let dy3 = state.stack.remove(0);
                    let c1x = state.x;
                    let c1y = state.y + dy1;
                    let c2x = c1x + dx2;
                    let c2y = c1y + dy2;
                    state.y = c2y + dy3;
                    state.commands.push(PathCommand::CurveTo {
                        c1x, c1y, c2x, c2y, x: state.x, y: state.y,
                    });
                }
                horizontal = !horizontal;
            }
            state.clear_stack();
        }
        10 => {
            // callsubr
            if let Some(&idx) = state.stack.last() {
                let idx = idx as i32 + state.local_subrs.len() as i32 / 2;
                let idx = idx as usize;
                state.stack.pop();
                if idx < state.local_subrs.len() {
                    decode_stream(&state.local_subrs[idx].clone(), state, depth + 1)?;
                }
            }
        }
        29 => {
            // callgsubr
            if let Some(&idx) = state.stack.last() {
                let idx = idx as i32 + state.global_subrs.len() as i32 / 2;
                let idx = idx as usize;
                state.stack.pop();
                if idx < state.global_subrs.len() {
                    decode_stream(&state.global_subrs[idx].clone(), state, depth + 1)?;
                }
            }
        }
        _ => {
            // Unknown operator — clear stack and continue
            state.clear_stack();
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_rmoveto_rlineto_endchar() {
        // vmoveto(4): dy=50
        // rlineto(5): dx=30, dy=20
        let data = vec![
            50 + 139, // 50
            4,        // vmoveto
            30 + 139, // 30
            20 + 139, // 20
            5,        // rlineto
            14,       // endchar
        ];
        let outline = decode_charstring(&data, &[], &[]).unwrap();
        assert_eq!(outline.commands.len(), 2);
        assert!(matches!(outline.commands[0], PathCommand::MoveTo { x: 0.0, y: 50.0 }));
        assert!(matches!(outline.commands[1], PathCommand::LineTo { x: 30.0, y: 70.0 }));
    }

    #[test]
    fn test_decode_hmoveto_hlineto() {
        // hmoveto(22): dx=50 (with width=80)
        // hlineto(6): dx=40
        let data = vec![
            80 + 139, // 80 (width)
            50 + 139, // 50
            22,       // hmoveto
            40 + 139, // 40
            6,        // hlineto
            14,       // endchar
        ];
        let outline = decode_charstring(&data, &[], &[]).unwrap();
        assert_eq!(outline.width, Some(80.0));
        assert_eq!(outline.commands.len(), 2);
        assert!(matches!(outline.commands[0], PathCommand::MoveTo { x: 50.0, y: 0.0 }));
        assert!(matches!(outline.commands[1], PathCommand::LineTo { x: 90.0, y: 0.0 }));
    }

    #[test]
    fn test_decode_rrcurveto() {
        // rrcurveto(8): dx1,dy1,dx2,dy2,dx3,dy3 = 10,20,5,10,15,20
        let data = vec![
            10 + 139, 20 + 139,
            5 + 139, 10 + 139,
            15 + 139, 20 + 139,
            8,         // rrcurveto
            14,        // endchar
        ];
        let outline = decode_charstring(&data, &[], &[]).unwrap();
        assert_eq!(outline.commands.len(), 1);
        if let PathCommand::CurveTo { c1x, c1y, c2x, c2y, x, y } = outline.commands[0] {
            assert_eq!(c1x, 10.0); assert_eq!(c1y, 20.0);
            assert_eq!(c2x, 15.0); assert_eq!(c2y, 30.0);
            assert_eq!(x, 30.0);   assert_eq!(y, 50.0);
        } else {
            panic!("Expected CurveTo");
        }
    }

    #[test]
    fn test_decode_empty_charstring() {
        let data = vec![14]; // endchar only
        let outline = decode_charstring(&data, &[], &[]).unwrap();
        assert_eq!(outline.commands.len(), 0);
        assert_eq!(outline.width, None);
    }

    #[test]
    fn test_shortint_and_negative() {
        // shortint(28): val=-500
        // vmoveto(4)
        let data = vec![
            28,        // shortint
            0xFE, 0x0C, // -500 (0xFE0C = -500 in i16)
            4,         // vmoveto
            14,        // endchar
        ];
        let outline = decode_charstring(&data, &[], &[]).unwrap();
        assert_eq!(outline.commands.len(), 1);
        assert!(matches!(outline.commands[0], PathCommand::MoveTo { x: 0.0, y: -500.0 }));
    }

    #[test]
    fn test_hint_operators_consume_stack() {
        // width=80, hstem: y=50, dy=20
        // vmoveto: dy=40
        let data = vec![
            80 + 139, // 80 (width)
            50 + 139, // 50
            20 + 139, // 20
            1,        // hstem
            40 + 139, // 40
            4,        // vmoveto
            14,       // endchar
        ];
        let outline = decode_charstring(&data, &[], &[]).unwrap();
        assert_eq!(outline.width, Some(80.0));
        assert_eq!(outline.commands.len(), 1);
        assert!(matches!(outline.commands[0], PathCommand::MoveTo { x: 0.0, y: 40.0 }));
    }
}

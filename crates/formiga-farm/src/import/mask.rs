//! A picture's cells as a mask: its pieces, and the mask shrunk, grown, trimmed and filled.

/// Every piece of `cells`, joined side to side, as lists of indices.
pub(super) fn pieces(cells: &[bool], width: usize, height: usize) -> Vec<Vec<usize>> {
    let mut seen = vec![false; cells.len()];
    let mut found = Vec::new();
    for start in 0..cells.len() {
        if !cells[start] || seen[start] {
            continue;
        }
        let mut piece = Vec::new();
        let mut stack = vec![start];
        seen[start] = true;
        while let Some(i) = stack.pop() {
            piece.push(i);
            let (x, y) = (i % width, i / width);
            for (nx, ny) in [
                (x.wrapping_sub(1), y),
                (x + 1, y),
                (x, y.wrapping_sub(1)),
                (x, y + 1),
            ] {
                if nx >= width || ny >= height {
                    continue;
                }
                let n = ny * width + nx;
                if cells[n] && !seen[n] {
                    seen[n] = true;
                    stack.push(n);
                }
            }
        }
        found.push(piece);
    }
    found
}

/// Only the largest piece of `cells`.
pub(super) fn largest_piece(cells: &[bool], width: usize, height: usize) -> Vec<bool> {
    let mut out = vec![false; cells.len()];
    if let Some(piece) = pieces(cells, width, height)
        .into_iter()
        .max_by_key(Vec::len)
    {
        for i in piece {
            out[i] = true;
        }
    }
    out
}

/// `cells` without whatever thin thing runs off the picture's edge, as the branch a bird sits
/// on or the line of the ground does, keeping thin parts of the subject itself, as its tail
/// or legs.
pub(super) fn trim(cells: &[bool], width: usize, height: usize) -> Vec<bool> {
    let count = cells.iter().filter(|c| **c).count();
    let radius = ((count as f32).sqrt() / 25.0).round().max(1.0) as usize;
    let thick = dilate(&erode(cells, width, height, radius), width, height, radius);
    let thick: Vec<bool> = thick.iter().zip(cells).map(|(t, c)| *t && *c).collect();
    if thick.iter().filter(|c| **c).count() < count / 3 {
        // The subject is all thin parts: keep it as it is.
        return cells.to_vec();
    }
    let mut out = largest_piece(&thick, width, height);
    let thin: Vec<bool> = cells.iter().zip(&thick).map(|(c, t)| *c && !t).collect();
    for piece in pieces(&thin, width, height) {
        let touches_edge = piece.iter().any(|&i| {
            let (x, y) = (i % width, i / width);
            x == 0 || y == 0 || x + 1 == width || y + 1 == height
        });
        if !touches_edge {
            for i in piece {
                out[i] = true;
            }
        }
    }
    largest_piece(&out, width, height)
}

/// `cells` with everything within `radius` of an empty cell (or the edge) emptied.
pub(super) fn erode(cells: &[bool], width: usize, height: usize, radius: usize) -> Vec<bool> {
    let r = radius as isize;
    let across: Vec<bool> = (0..cells.len())
        .map(|i| {
            let (x, y) = ((i % width) as isize, i / width);
            (-r..=r).all(|d| {
                let nx = x + d;
                nx >= 0 && (nx as usize) < width && cells[y * width + nx as usize]
            })
        })
        .collect();
    (0..cells.len())
        .map(|i| {
            let (x, y) = (i % width, (i / width) as isize);
            (-r..=r).all(|d| {
                let ny = y + d;
                ny >= 0 && (ny as usize) < height && across[ny as usize * width + x]
            })
        })
        .collect()
}

/// `cells` with everything within `radius` of a filled cell filled.
pub(super) fn dilate(cells: &[bool], width: usize, height: usize, radius: usize) -> Vec<bool> {
    let outside: Vec<bool> = cells.iter().map(|c| !c).collect();
    // Dilating is eroding the outside; the picture's edge counts as inside here.
    let r = radius as isize;
    let across: Vec<bool> = (0..cells.len())
        .map(|i| {
            let (x, y) = ((i % width) as isize, i / width);
            (-r..=r).all(|d| {
                let nx = x + d;
                nx < 0 || nx as usize >= width || outside[y * width + nx as usize]
            })
        })
        .collect();
    (0..cells.len())
        .map(|i| {
            let (x, y) = (i % width, (i / width) as isize);
            !(-r..=r).all(|d| {
                let ny = y + d;
                ny < 0 || ny as usize >= height || across[ny as usize * width + x]
            })
        })
        .collect()
}

/// `cells` with every hole that does not reach the edge filled in.
pub(super) fn fill_holes(cells: &[bool], width: usize, height: usize) -> Vec<bool> {
    let outside: Vec<bool> = cells.iter().map(|c| !c).collect();
    let mut out = cells.to_vec();
    for piece in pieces(&outside, width, height) {
        let touches_edge = piece.iter().any(|&i| {
            let (x, y) = (i % width, i / width);
            x == 0 || y == 0 || x + 1 == width || y + 1 == height
        });
        if !touches_edge {
            for i in piece {
                out[i] = true;
            }
        }
    }
    out
}

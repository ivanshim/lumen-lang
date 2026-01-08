// Lumen Structure Processor
// Provides semantic interpretation for Lumen indentation-based blocks

use crate::kernel::{OpaqueAnalysis, StructureProcessor, Token};

/// Lumen's internal semantic data
/// Kernel will never see this type - it's completely opaque
#[derive(Debug, Clone)]
pub struct LumenAnalysis {
    pub depth: i32,
    pub is_newline: bool,
}

/// Lumen's structure processor implementation
pub struct LumenStructureProcessor;

impl StructureProcessor for LumenStructureProcessor {
    fn analyze(&self, token: &Token) -> OpaqueAnalysis {
        let mut analysis = LumenAnalysis {
            depth: 0,
            is_newline: false,
        };

        // Lumen interprets tokens
        if token.name == "indent" {
            // Lumen semantic: 4 spaces = 1 depth level
            let spaces = token.lexeme.len();
            analysis.depth = (spaces / 4) as i32;
        }

        if token.name == "newline" {
            analysis.is_newline = true;
        }

        // Return as opaque - kernel can't see what's inside
        Box::new(analysis)
    }

    fn handle_change(
        &self,
        prev_analysis: Option<&OpaqueAnalysis>,
        curr_analysis: &OpaqueAnalysis,
    ) -> Vec<Token> {
        let mut inserted = Vec::new();

        // Language downcasts opaque data to its semantic type
        let curr = match curr_analysis.downcast_ref::<LumenAnalysis>() {
            Some(analysis) => analysis,
            None => return inserted,
        };

        let prev = prev_analysis
            .and_then(|a| a.downcast_ref::<LumenAnalysis>())
            .map(|a| a.clone());

        // Lumen's semantic rules: when depth changes, insert markers
        if let Some(prev) = prev {
            if curr.depth > prev.depth {
                // Lumen semantic: depth increase → block starts
                let levels_increased = curr.depth - prev.depth;
                for _ in 0..levels_increased {
                    inserted.push(Token::new("marker_block_start", "{"));
                }
            } else if curr.depth < prev.depth {
                // Lumen semantic: depth decrease → block ends
                let levels_decreased = prev.depth - curr.depth;
                for _ in 0..levels_decreased {
                    inserted.push(Token::new("marker_block_end", "}"));
                }
            }
        }

        inserted
    }
}

/// Factory function for Lumen structure processor
pub fn lumen_structure() -> Box<dyn StructureProcessor> {
    Box::new(LumenStructureProcessor)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lumen_analyzes_indent() {
        let processor = LumenStructureProcessor;
        let token = Token::new("indent", "    "); // 4 spaces

        let analysis = processor.analyze(&token);
        let lumen_analysis = analysis.downcast_ref::<LumenAnalysis>().unwrap();

        assert_eq!(lumen_analysis.depth, 1);
    }

    #[test]
    fn test_lumen_handles_depth_increase() {
        let processor = LumenStructureProcessor;

        let prev: OpaqueAnalysis = Box::new(LumenAnalysis {
            depth: 0,
            is_newline: false,
        });
        let curr: OpaqueAnalysis = Box::new(LumenAnalysis {
            depth: 1,
            is_newline: false,
        });

        let inserted = processor.handle_change(Some(&prev), &curr);

        assert_eq!(inserted.len(), 1);
        assert_eq!(inserted[0].name, "marker_block_start");
    }

    #[test]
    fn test_lumen_handles_depth_decrease() {
        let processor = LumenStructureProcessor;

        let prev: OpaqueAnalysis = Box::new(LumenAnalysis {
            depth: 2,
            is_newline: false,
        });
        let curr: OpaqueAnalysis = Box::new(LumenAnalysis {
            depth: 0,
            is_newline: false,
        });

        let inserted = processor.handle_change(Some(&prev), &curr);

        assert_eq!(inserted.len(), 2);
        assert_eq!(inserted[0].name, "marker_block_end");
        assert_eq!(inserted[1].name, "marker_block_end");
    }
}

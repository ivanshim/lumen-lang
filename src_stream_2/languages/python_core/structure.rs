// Python Core Structure Processor
// Similar to Lumen but with colon handling

use crate::kernel::{OpaqueAnalysis, StructureProcessor, Token};

#[derive(Debug, Clone)]
pub struct PythonAnalysis {
    pub depth: i32,
    pub has_colon: bool,
}

pub struct PythonStructureProcessor;

impl StructureProcessor for PythonStructureProcessor {
    fn analyze(&self, token: &Token) -> OpaqueAnalysis {
        let mut analysis = PythonAnalysis {
            depth: 0,
            has_colon: false,
        };

        if token.name == "indent" {
            // Python: 4 spaces per level (convention)
            analysis.depth = (token.lexeme.len() / 4) as i32;
        }

        if token.name == "colon" {
            analysis.has_colon = true;
        }

        Box::new(analysis)
    }

    fn handle_change(
        &self,
        prev_analysis: Option<&OpaqueAnalysis>,
        curr_analysis: &OpaqueAnalysis,
    ) -> Vec<Token> {
        let mut inserted = Vec::new();

        let curr = match curr_analysis.downcast_ref::<PythonAnalysis>() {
            Some(analysis) => analysis,
            None => return inserted,
        };

        let prev = prev_analysis
            .and_then(|a| a.downcast_ref::<PythonAnalysis>())
            .map(|a| a.clone());

        // Python's depth change rules (similar to Lumen)
        if let Some(prev) = prev {
            if curr.depth > prev.depth {
                let levels = curr.depth - prev.depth;
                for _ in 0..levels {
                    inserted.push(Token::new("marker_block_start", ":"));
                }
            } else if curr.depth < prev.depth {
                let levels = prev.depth - curr.depth;
                for _ in 0..levels {
                    inserted.push(Token::new("marker_block_end", "}"));
                }
            }
        }

        inserted
    }
}

pub fn python_structure() -> Box<dyn StructureProcessor> {
    Box::new(PythonStructureProcessor)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_python_analyzes_indent() {
        let processor = PythonStructureProcessor;
        let token = Token::new("indent", "        "); // 8 spaces = 2 levels

        let analysis = processor.analyze(&token);
        let py_analysis = analysis.downcast_ref::<PythonAnalysis>().unwrap();

        assert_eq!(py_analysis.depth, 2);
    }

    #[test]
    fn test_python_detects_colon() {
        let processor = PythonStructureProcessor;
        let token = Token::new("colon", ":");

        let analysis = processor.analyze(&token);
        let py_analysis = analysis.downcast_ref::<PythonAnalysis>().unwrap();

        assert_eq!(py_analysis.has_colon, true);
    }
}

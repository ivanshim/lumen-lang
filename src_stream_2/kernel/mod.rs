// Opaque Analysis Kernel
// Completely semantic-free processing engine
// Language provides: analyze() method and handle_change() method
// Kernel does: the iteration and orchestration

use std::any::Any;

#[derive(Debug, Clone)]
pub struct Token {
    pub name: String,
    pub lexeme: String,
}

impl Token {
    pub fn new(name: impl Into<String>, lexeme: impl Into<String>) -> Self {
        Token {
            name: name.into(),
            lexeme: lexeme.into(),
        }
    }
}

/// Opaque analysis data - kernel has ZERO knowledge what's inside
pub type OpaqueAnalysis = Box<dyn Any + Send + Sync>;

/// Language provides semantic interpretation through this trait
pub trait StructureProcessor: Send + Sync {
    /// Analyze a token and extract language-specific properties
    /// Returns opaque data that kernel will pass around
    fn analyze(&self, token: &Token) -> OpaqueAnalysis;

    /// Handle change between previous and current analysis
    /// Language decides what's meaningful, what to insert
    /// Kernel just asks and uses the result
    fn handle_change(
        &self,
        prev_analysis: Option<&OpaqueAnalysis>,
        curr_analysis: &OpaqueAnalysis,
    ) -> Vec<Token>;
}

/// The kernel itself - completely dumb processor
pub struct Kernel {
    processor: Box<dyn StructureProcessor>,
}

impl Kernel {
    pub fn new(processor: Box<dyn StructureProcessor>) -> Self {
        Kernel { processor }
    }

    /// The kernel's only job: orchestrate token processing
    /// Step 1: Iterate tokens
    /// Step 2: Ask language to analyze each
    /// Step 3: Ask language to handle changes
    /// Step 4: Insert result tokens
    /// Kernel has ZERO semantic knowledge
    pub fn process_structure(&self, tokens: Vec<Token>) -> Result<Vec<Token>, String> {
        let mut result = Vec::new();
        let mut prev_analysis: Option<OpaqueAnalysis> = None;

        for token in tokens {
            // 1. Kernel asks language: what does this token mean?
            let curr_analysis = self.processor.analyze(&token);

            // 2. Kernel asks language: what should we do?
            let inserted = self.processor.handle_change(prev_analysis.as_ref(), &curr_analysis);

            // 3. Kernel mechanically applies the result
            result.extend(inserted);
            result.push(token);

            // 4. Kernel updates state for next iteration
            prev_analysis = Some(curr_analysis);
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test that kernel is completely dumb and doesn't care about semantics
    struct TestProcessor;

    impl StructureProcessor for TestProcessor {
        fn analyze(&self, token: &Token) -> OpaqueAnalysis {
            // Just echo back something
            Box::new(token.name.clone())
        }

        fn handle_change(
            &self,
            _prev: Option<&OpaqueAnalysis>,
            _curr: &OpaqueAnalysis,
        ) -> Vec<Token> {
            // Never insert anything
            vec![]
        }
    }

    #[test]
    fn test_kernel_processes_tokens() {
        let kernel = Kernel::new(Box::new(TestProcessor));
        let tokens = vec![
            Token::new("a", "a"),
            Token::new("b", "b"),
            Token::new("c", "c"),
        ];

        let result = kernel.process_structure(tokens).unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].name, "a");
        assert_eq!(result[1].name, "b");
        assert_eq!(result[2].name, "c");
    }

    #[test]
    fn test_kernel_inserts_tokens() {
        struct InsertingProcessor;

        impl StructureProcessor for InsertingProcessor {
            fn analyze(&self, _token: &Token) -> OpaqueAnalysis {
                Box::new(true)
            }

            fn handle_change(
                &self,
                _prev: Option<&OpaqueAnalysis>,
                _curr: &OpaqueAnalysis,
            ) -> Vec<Token> {
                vec![Token::new("inserted", "X")]
            }
        }

        let kernel = Kernel::new(Box::new(InsertingProcessor));
        let tokens = vec![Token::new("a", "a"), Token::new("b", "b")];

        let result = kernel.process_structure(tokens).unwrap();
        // Should be: inserted, a, inserted, b
        assert_eq!(result.len(), 4);
        assert_eq!(result[0].name, "inserted");
        assert_eq!(result[1].name, "a");
        assert_eq!(result[2].name, "inserted");
        assert_eq!(result[3].name, "b");
    }
}

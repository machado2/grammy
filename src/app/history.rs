use std::collections::VecDeque;

/// A single message in the conversation history.
#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub role: String, // "user" or "assistant"
    pub content: String,
}

/// Tracks the last N user/assistant message pairs to provide context to the LLM.
/// This helps avoid repetitive or cyclic suggestions.
#[derive(Debug)]
pub struct MessageHistory {
    entries: VecDeque<HistoryEntry>,
    max_pairs: usize, // Maximum number of user/assistant pairs
}

impl Default for MessageHistory {
    fn default() -> Self {
        Self::new(5)
    }
}

impl MessageHistory {
    /// Create a new history with capacity for `max_pairs` user/assistant pairs.
    pub fn new(max_pairs: usize) -> Self {
        Self {
            entries: VecDeque::with_capacity(max_pairs * 2),
            max_pairs,
        }
    }

    /// Add a user/assistant message pair to the history.
    pub fn push_pair(&mut self, user_content: String, assistant_content: String) {
        // Remove oldest pair if at capacity
        while self.entries.len() >= self.max_pairs * 2 {
            self.entries.pop_front(); // Remove user
            self.entries.pop_front(); // Remove assistant
        }

        self.entries.push_back(HistoryEntry {
            role: "user".to_string(),
            content: user_content,
        });
        self.entries.push_back(HistoryEntry {
            role: "assistant".to_string(),
            content: assistant_content,
        });
    }

    /// Get all history entries for inclusion in API request.
    pub fn get_entries(&self) -> Vec<&HistoryEntry> {
        self.entries.iter().collect()
    }

    /// Clear all history (e.g., when starting fresh).
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Check if history is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_and_get() {
        let mut history = MessageHistory::new(3);
        history.push_pair("user1".into(), "assistant1".into());
        history.push_pair("user2".into(), "assistant2".into());

        let entries = history.get_entries();
        assert_eq!(entries.len(), 4);
        assert_eq!(entries[0].role, "user");
        assert_eq!(entries[0].content, "user1");
        assert_eq!(entries[1].role, "assistant");
        assert_eq!(entries[1].content, "assistant1");
    }

    #[test]
    fn test_capacity_limit() {
        let mut history = MessageHistory::new(2);
        history.push_pair("user1".into(), "assistant1".into());
        history.push_pair("user2".into(), "assistant2".into());
        history.push_pair("user3".into(), "assistant3".into());

        let entries = history.get_entries();
        assert_eq!(entries.len(), 4); // 2 pairs = 4 entries
        assert_eq!(entries[0].content, "user2"); // Oldest pair removed
    }

    #[test]
    fn test_clear() {
        let mut history = MessageHistory::new(5);
        history.push_pair("user1".into(), "assistant1".into());
        history.clear();
        assert!(history.is_empty());
    }
}

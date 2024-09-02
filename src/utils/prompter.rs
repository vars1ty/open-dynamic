use smallvec::SmallVec;
use zstring::ZString;

/// Basic CLI Prompter.
pub struct Prompter<'a> {
    message: &'a str,
    valid_responses: Option<SmallVec<[&'a str; 4]>>,
}

/// Prompt result.
pub struct PromptResult {
    pub prompt: ZString,
    pub args: SmallVec<[String; 4]>,
}

impl<'a> Prompter<'a> {
    /// Creates a new prompt which only classifies certain responses as valid.
    #[allow(unused)]
    pub fn new(message: &'a str, valid_responses: SmallVec<[&'a str; 4]>) -> Self {
        Self {
            message,
            valid_responses: Some(valid_responses),
        }
    }

    /// Creates a new prompt which classifies all responses as valid ones.
    pub fn new_any_response(message: &'a str) -> Self {
        Self {
            message,
            valid_responses: None,
        }
    }

    /// Prompts the user.
    pub fn prompt(&mut self) -> Option<PromptResult> {
        log!(self.message);

        let mut input = ZString::default();
        std::io::stdin()
            .read_line(&mut input.data)
            .unwrap_or_default();

        // Remove new lines and carriage return symbols.
        input.data = input.data.replace(['\n', '\r'], "");

        // Collect all args, if any.
        let mut args: SmallVec<[String; 4]> = if input.data.contains(' ') {
            input
                .data
                .split_whitespace()
                .map(|str| str.to_owned())
                .collect()
        } else {
            smallvec![input.data.to_owned()]
        };

        // The prompt is the first argument, no need for cloning it since we can just take the
        // string as-is via std::mem::take().
        let prompt = ZString::new(std::mem::take(&mut args[0]));

        // If there's any valid responses set, check if it's valid and return the response.
        // Otherwise just return the input without checking if it's valid.
        if let Some(responses) = &self.valid_responses {
            if responses.contains(&input.data.as_str()) {
                // as_str is fucking stupid, but its needed
                // this time.
                return Some(PromptResult { prompt, args });
            } else {
                return None;
            }
        }

        // Any response is valid, return everything.
        Some(PromptResult { prompt, args })
    }

    /// Call if the response was invalid.
    /// Only executes if there's any valid responses set.
    #[allow(unused)]
    pub fn print_invalid_usage(&self) {
        let Some(ref valid_responses) = self.valid_responses else {
            return;
        };

        log!(
            "Invalid Usage! Valid responses are: ",
            valid_responses.join(",")
        )
    }
}

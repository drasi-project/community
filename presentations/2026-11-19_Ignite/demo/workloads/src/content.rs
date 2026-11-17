//! Simulated request content. The questions and answers are synthetic but
//! realistic, so a Langfuse trace reads like a real AI support request.

const SUPPORT: [(&str, &str); 8] = [
    ("I was charged twice for my subscription this month. Can you refund one of the charges?",
     "I'm sorry about the double charge. I've flagged the duplicate payment for a refund; it will appear on your statement within 5 business days."),
    ("I can't sign in after resetting my password. It says my account is locked.",
     "Your account was locked after several failed attempts. I've sent an unlock link to your email; use it, then sign in with your new password."),
    ("My order hasn't arrived and the tracking hasn't updated for a week.",
     "The carrier has your parcel at a regional hub. I've opened a trace with them and you'll get an update within 48 hours, or a free replacement."),
    ("How do I export my data before I close my account?",
     "Go to Settings, then Privacy, then Export data. You'll get a download link by email, usually within an hour."),
    ("The app crashes every time I open the reports page.",
     "This is a known issue in version 4.2. Please update to 4.2.1 from the app store; it fixes the reports page crash."),
    ("Can I move my team from the monthly plan to the annual plan?",
     "Yes. Go to Billing, then Change plan, and choose Annual. You'll be credited for the unused part of this month."),
    ("I need an invoice with our company's VAT number on it.",
     "I've added your VAT number to your billing profile and reissued last month's invoice. You'll find it under Billing, then Invoices."),
    ("Two-factor codes aren't reaching my phone.",
     "Text messages to your carrier are delayed today. You can sign in with a backup code, or switch to an authenticator app in Security settings."),
];

const SUMMARIES: [(&str, &str); 4] = [
    ("Summarize the public release notes for version 4.2.",
     "Version 4.2 adds scheduled reports, faster search and a redesigned billing page, and fixes 23 bugs."),
    ("Summarize the public help article on setting up single sign-on.",
     "Single sign-on is configured under Security: add your identity provider's metadata, map user groups, then test with one account before enabling it for everyone."),
    ("Summarize the public status page history for last month.",
     "Last month had two incidents: a 20-minute sign-in delay and a reporting slowdown. Both were resolved, with 99.95% availability overall."),
    ("Summarize the public product roadmap blog post.",
     "The roadmap focuses on offline mode, more integrations and AI-assisted reporting over the next two quarters."),
];

/// The request text for the `index`th generated request of a workload.
pub fn question(workload: &str, index: usize) -> &'static str {
    if workload == "support" {
        SUPPORT[index % SUPPORT.len()].0
    } else {
        SUMMARIES[index % SUMMARIES.len()].0
    }
}

/// The simulated model's answer to a request.
pub fn answer(workload: &str, content: &str) -> String {
    let known = if workload == "support" {
        &SUPPORT[..]
    } else {
        &SUMMARIES[..]
    };
    match known.iter().find(|(q, _)| *q == content) {
        Some((_, a)) => (*a).to_owned(),
        None if workload == "support" => {
            "Thanks for getting in touch. I've looked into your request and sent the next steps to your email.".to_owned()
        }
        None => "Summary: the document describes recent product changes and how to use them.".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_generated_question_has_a_specific_answer() {
        for i in 0..8 {
            let q = question("support", i);
            assert_ne!(answer("support", q), answer("support", "something else"));
        }
        for i in 0..4 {
            let q = question("batch", i);
            assert_ne!(answer("batch", q), answer("batch", "something else"));
        }
    }
}

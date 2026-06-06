                        if output.status.success() {
                            let pr_url = String::from_utf8(output.stdout)?.trim().to_string();
                            let action = if check_output.stdout.is_empty() || check_output.stdout == b"[]\n" { "created" } else { "updated" };
                            final_response = format!("{}\n\nPull Request {}: {}", final_response, action, pr_url);
                        } else {
                            // If PR already exists, try to update it? For now, just report error.
                            return Err(anyhow!("Failed to manage PR: {}", String::from_utf8_lossy(&output.stderr)));
                        }

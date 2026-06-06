                        if output.status.success() {
                            let pr_url = String::from_utf8(output.stdout)?.trim().to_string();
                            let action = if check_output.stdout.is_empty() || check_output.stdout == b"[]
" { "created" } else { "updated" };
                            return Err(anyhow!("Failed to manage PR: {}", String::from_utf8_lossy(&output.stderr)));
                        }

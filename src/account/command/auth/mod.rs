pub mod config;
pub mod error;
pub mod flow;
pub mod provider;

use std::path::Path;
use std::io::{self, Write};
use color_eyre::Result;

use self::config::ConfigWriter;
use self::flow::OAuthFlow;
use self::provider::AuthProvider;

/// OAuth authentication orchestrator
///
/// Coordinates the OAuth 2.0 flow: browser opening, callback handling, token exchange,
/// and configuration writing.
pub struct OAuthAuthenticator {
    provider: AuthProvider,
    account_name: String,
    email: String,
    client_id: String,
    client_secret: String,
}

impl OAuthAuthenticator {
    /// Create a new OAuth authenticator
    pub fn new(
        provider: AuthProvider,
        account_name: String,
        email: String,
        client_id: String,
        client_secret: String,
    ) -> Self {
        Self {
            provider,
            account_name,
            email,
            client_id,
            client_secret,
        }
    }

    /// Execute the complete OAuth authentication flow
    pub async fn authenticate(&self, config_path: &Path) -> Result<()> {
        println!("\n🔐 Starting {} OAuth setup", self.provider);
        println!("Account: {}", self.account_name);
        println!("Email: {}", self.email);

        // Step 1: Execute OAuth flow (browser, callback, token exchange)
        let flow = OAuthFlow::new(
            self.provider,
            self.account_name.clone(),
            self.client_id.clone(),
            self.client_secret.clone(),
        );

        let tokens = flow
            .execute()
            .await
            .map_err(|e| {
                eprintln!("❌ OAuth flow failed: {}", e);
                color_eyre::eyre::eyre!("{}", e)
            })?;

        // Step 2: Write configuration and store tokens
        let config_writer = ConfigWriter::new(
            self.account_name.clone(),
            self.email.clone(),
            self.provider,
        );

        config_writer
            .write_config(config_path, tokens)
            .await
            .map_err(|e| {
                eprintln!("❌ Failed to write configuration: {}", e);
                color_eyre::eyre::eyre!("{}", e)
            })?;

        // Step 3: Validate the setup by checking keyring access
        println!("\n🧪 Validating account setup...");
        self.validate_setup().await;

        println!("\n✅ OAuth setup complete!");
        println!("Account '{}' is ready to use.", self.account_name);
        println!("\nYou can now use Himalaya to access your email:");
        println!("  himalaya account list");
        println!("  himalaya envelope list");

        Ok(())
    }

    /// Validate that the OAuth setup was successful by checking token storage
    async fn validate_setup(&self) {
        #[cfg(feature = "keyring")]
        {
            use secret::Secret;

            let imap_access_token_key = format!("{}-imap-access-token", self.account_name);

            // Try to create a keyring entry and retrieve the token
            match secret::keyring::KeyringEntry::try_new(&imap_access_token_key) {
                Ok(entry) => {
                    // Try to retrieve the token to verify it was stored
                    let mut secret = Secret::new_keyring_entry(entry);
                    match secret.find().await {
                        Ok(Some(_)) => {
                            println!("✓ Configuration validated");
                            println!("✓ OAuth tokens securely stored in system keyring");
                        }
                        Ok(None) => {
                            eprintln!("⚠️  Warning: Access token not found in keyring");
                            eprintln!("    This may happen if the keyring wasn't available during setup.");
                            eprintln!("    Try: himalaya account doctor {} --fix", self.account_name);
                        }
                        Err(e) => {
                            eprintln!("⚠️  Warning: Could not verify keyring access: {}", e);
                            eprintln!("    The keyring may be locked or the account may not work properly.");
                            eprintln!("    Try: himalaya account doctor {} --fix", self.account_name);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("⚠️  Warning: Could not access keyring system: {}", e);
                    eprintln!("    The account may not work without keyring access.");
                    eprintln!("    Try: himalaya account doctor {} --fix", self.account_name);
                }
            }
        }

        #[cfg(not(feature = "keyring"))]
        {
            eprintln!("⚠️  Warning: Keyring feature not enabled. Tokens are not stored securely.");
            eprintln!("    Install with keyring feature for secure token storage:");
            eprintln!("    cargo install himalaya --features oauth2,keyring");
        }
    }
}

/// OAuth authentication CLI command
use clap::Parser;

/// Authenticate with an OAuth 2.0 provider
///
/// This command sets up OAuth 2.0 authentication for a supported email provider
/// (Gmail, Outlook, etc.). It will open your browser for authorization and
/// automatically configure your account.
#[derive(Debug, Parser)]
pub struct AccountAuthCommand {
    /// OAuth provider (gmail, outlook, yahoo)
    #[arg(value_name = "PROVIDER")]
    pub provider: String,

    /// Account name (optional - defaults to provider name)
    #[arg(value_name = "ACCOUNT_NAME")]
    pub account_name: Option<String>,
}

impl AccountAuthCommand {
    pub async fn execute(
        self,
        _config: crate::config::TomlConfig,
        config_path: Option<&std::path::PathBuf>,
    ) -> Result<()> {
        use tracing::info;
        use pimalaya_tui::terminal::prompt;

        info!("executing account auth command");

        // Parse and validate provider
        let provider = provider::AuthProvider::from_str(&self.provider)
            .ok_or_else(|| {
                color_eyre::eyre::eyre!(
                    "Unsupported OAuth provider: '{}'\nSupported providers: gmail",
                    self.provider
                )
            })?;

        // Determine account name
        let account_name = if let Some(name) = self.account_name {
            name
        } else {
            // Use provider name as default
            let default_name = self.provider.to_lowercase();

            // Prompt user to confirm or override
            print!("Account name [default: {}]: ", default_name);
            io::stdout().flush()?;
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let trimmed = input.trim();
            if trimmed.is_empty() {
                default_name
            } else {
                trimmed.to_string()
            }
        };

        // Prompt for client credentials
        println!("\nPlease provide your OAuth 2.0 credentials:");
        println!("(See https://github.com/pimalaya/himalaya#oauth-setup for instructions)\n");

        print!("Client ID: ");
        io::stdout().flush()?;
        let mut client_id = String::new();
        io::stdin().read_line(&mut client_id)?;
        let client_id = client_id.trim().to_string();

        let client_secret = prompt::password("Client Secret")?;

        // Prompt for email address
        print!("Email address: ");
        io::stdout().flush()?;
        let mut email = String::new();
        io::stdin().read_line(&mut email)?;
        let email = email.trim().to_string();

        // Get config path
        let config_path = match config_path {
            Some(path) => path.clone(),
            None => {
                use pimalaya_tui::terminal::config::TomlConfig;
                crate::config::TomlConfig::default_path()?
            }
        };

        // Create authenticator and run OAuth flow
        let authenticator = OAuthAuthenticator::new(
            provider,
            account_name,
            email,
            client_id,
            client_secret,
        );

        authenticator.authenticate(&config_path).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_authenticator_creation() {
        let auth = OAuthAuthenticator::new(
            AuthProvider::Gmail,
            "gmail".to_string(),
            "test@gmail.com".to_string(),
            "client-id".to_string(),
            "client-secret".to_string(),
        );
        
        assert_eq!(auth.account_name, "gmail");
        assert_eq!(auth.email, "test@gmail.com");
    }
}

# post-on-linkedin

A lightweight Rust CLI tool to publish text and image posts to your personal LinkedIn profile or any LinkedIn Organization/Company Page you manage, configured via `config.yaml`.

---

## How to Get Your LinkedIn Access Token & IDs

### Step 1: Create a LinkedIn Developer App
1. Go to the [LinkedIn Developer Portal](https://www.linkedin.com/developers/apps) and click **Create app**.
2. Fill in the **App name**, link your **LinkedIn Page** (required by LinkedIn to create an app), upload an app logo, and click **Create app**.
3. Open the **Settings** tab of your new app and click **Verify** next to your LinkedIn Page so your app is associated with the page.

### Step 2: Enable the Required Products
Open the **Products** tab in your app settings and request access to:
- **Share on LinkedIn** — allows posting to your personal profile (`w_member_social`).
- **Sign In with LinkedIn using OpenID Connect** — allows looking up your personal profile ID (`openid`, `profile`).
- **Community Management API** *(or Advertising API / Marketing Developer Platform)* — required if you want to post on behalf of an Organization/Company Page (`w_organization_social`).

### Step 3: Generate Your Access Token (No Code Needed)
LinkedIn provides a built-in token generator in the Developer Portal:
1. Go to [LinkedIn OAuth 2.0 Tools](https://www.linkedin.com/developers/tools/oauth) and click **Create token**.
2. Select your app from the dropdown.
3. Check the following scopes:
   - `openid` and `profile` *(to fetch your `person_id`)*
   - `w_member_social` *(to post to your personal profile)*
   - `w_organization_social` *(if enabled, to post to your company page)*
4. Click **Request access token**, approve the prompt in the browser, and copy your generated **Access Token**.

### Step 4: Find Your `person_id` and Organization IDs
- **Personal Profile ID (`person_id`):**
  Paste your `access_token` into `config.yaml` and run:
  ```bash
  cargo run -- whoami
  ```
  It will print your `Person ID (sub)` (e.g., `abc123XYZ`). Copy that value into `person_id` in `config.yaml`.

- **Organization / Page ID:**
  Open your LinkedIn Company Page as an admin in your browser. Look at the URL in the address bar:
  ```text
  https://www.linkedin.com/company/12345678/admin/dashboard/
  ```
  The number in the URL (`12345678`) is your Organization ID. Add it under `organizations` in `config.yaml`.

---

## Quick Start

### 1. Configure `config.yaml`
Edit `config.yaml` (or copy from `config.example.yaml`):

```yaml
access_token: "AQV..."
api_version: "202601"
person_id: "YOUR_PERSON_ID"
organizations:
  my-company: "12345678"
```

### 2. List Configured Targets
Verify your profile and organization aliases:
```bash
cargo run -- targets
```

### 3. Post to Your Personal Profile (Text + Optional Image)
```bash
# Text only
cargo run -- post --text "Hello LinkedIn from Rust! 🦀"

# Text + Image
cargo run -- post --text "Hello LinkedIn from Rust! 🦀" --image ./photo.png --alt-text "Photo description"

# Multi-line text from a file
cargo run -- post --text-file ./post.txt --image ./photo.png
```

### 4. Post to a Managed Organization Page
Pass `--org <alias>` (matching a key under `organizations` in `config.yaml`, or a raw numeric Organization ID):
```bash
cargo run -- post --org my-company --text "We just shipped a new release!" --image ./banner.jpg
```

### 5. Preview Without Posting (`--dry-run`)
Add `--dry-run` to any `post` command to inspect the resolved Author URN and JSON payload without calling the LinkedIn API:
```bash
cargo run -- post --org my-company --text "Testing payload" --image ./banner.jpg --dry-run
```

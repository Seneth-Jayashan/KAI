import imaplib
import email
from email.header import decode_header
import json
import os
import sys

CONFIG_PATH = os.path.expanduser("~/.local/share/kai/config.json")

def read_emails(target_alias="all", limit=5):
    if not os.path.exists(CONFIG_PATH):
        print(f"Error: Config file not found at {CONFIG_PATH}")
        return

    with open(CONFIG_PATH, "r") as f:
        config = json.load(f)

    accounts = config.get("email", {}).get("accounts", [])
    if not accounts:
        print("Error: No email accounts configured in config.json")
        return

    accounts_to_check = accounts if target_alias == "all" else [acc for acc in accounts if acc.get("alias") == target_alias]

    if not accounts_to_check:
        print(f"Error: No account found with alias '{target_alias}'")
        return

    for cred in accounts_to_check:
        alias = cred.get("alias", "Unknown")
        user = cred.get("address")
        password = cred.get("app_password")
        imap_url = cred.get("imap_server")

        if not user or not password or "YOUR_" in user or "YOUR_" in password:
            print(f"Skipping {alias} (credentials not set up)")
            continue

        try:
            # connect to the server and go to its inbox
            mail = imaplib.IMAP4_SSL(imap_url)
            mail.login(user, password)
            mail.select("inbox")

            status, messages = mail.search(None, "ALL")
            if status != "OK":
                print(f"Failed to retrieve emails for {alias}.")
                continue

            email_ids = messages[0].split()
            latest_email_ids = email_ids[-limit:]
            
            if not latest_email_ids:
                print(f"Inbox is empty for {alias}.")
                continue

            print(f"--- LATEST {len(latest_email_ids)} EMAILS FROM {alias.upper()} ({user}) ---")
            
            for e_id in reversed(latest_email_ids):
                res, msg = mail.fetch(e_id, "(RFC822)")
                for response in msg:
                    if isinstance(response, tuple):
                        msg = email.message_from_bytes(response[1])
                        subject, encoding = decode_header(msg["Subject"])[0]
                        if isinstance(subject, bytes):
                            subject = subject.decode(encoding if encoding else "utf-8")
                        From, encoding = decode_header(msg.get("From"))[0]
                        if isinstance(From, bytes):
                            From = From.decode(encoding if encoding else "utf-8")
                        
                        print(f"\nFrom: {From}")
                        print(f"Subject: {subject}")
                        
                        if msg.is_multipart():
                            for part in msg.walk():
                                content_type = part.get_content_type()
                                if content_type == "text/plain":
                                    body = part.get_payload(decode=True).decode()
                                    print(f"Body:\n{body[:200]}...")
                                    break
                        else:
                            content_type = msg.get_content_type()
                            if content_type == "text/plain" or content_type == "text/html":
                                body = msg.get_payload(decode=True).decode()
                                print(f"Body:\n{body[:200]}...")
                print("-" * 40)
                
            mail.close()
            mail.logout()
        except Exception as e:
            print(f"Error connecting to {alias}: {e}")

if __name__ == "__main__":
    target = sys.argv[1] if len(sys.argv) > 1 else "all"
    limit = int(sys.argv[2]) if len(sys.argv) > 2 else 5
    read_emails(target, limit)

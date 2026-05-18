import os
import json
import uuid
import hashlib
import base64
import keyring
import platformdirs
from cryptography.fernet import Fernet, InvalidToken
from cryptography.hazmat.primitives.kdf.pbkdf2 import PBKDF2HMAC
from cryptography.hazmat.primitives import hashes

VAULT_DIR = platformdirs.user_data_dir("Lanyard", "LanyardApp")
META_FILE = os.path.join(VAULT_DIR, "meta.json")
CONFIG_FILE = os.path.join(VAULT_DIR, "config.json")
KEYRING_SERVICE = "Lanyard_Secure_Vault"

class Vault:
    def __init__(self):
        os.makedirs(VAULT_DIR, exist_ok=True)
        self.meta = self._load_json(META_FILE, {"items": [], "projects": []})
        self.config = self._load_json(CONFIG_FILE, {"salt": None, "pin_hash": None, "close_to_tray": True})
        self._encryption_key = None

    def _load_json(self, path, default):
        if os.path.exists(path):
            try:
                with open(path, "r", encoding="utf-8") as f:
                    return json.load(f)
            except Exception as e:
                print(f"Failed to load JSON {path}: {e}")
        return default

    def _save_json(self, path, data):
        with open(path, "w", encoding="utf-8") as f:
            json.dump(data, f, indent=2)

    # --- ENCRYPTION & PIN Management ---
    def _derive_key(self, pin: str, salt: bytes) -> bytes:
        kdf = PBKDF2HMAC(
            algorithm=hashes.SHA256(),
            length=32,
            salt=salt,
            iterations=480000,
        )
        return base64.urlsafe_b64encode(kdf.derive(pin.encode()))

    def set_pin(self, pin: str):
        salt = os.urandom(16)
        key = self._derive_key(pin, salt)
        pin_hash = hashlib.sha256(key).hexdigest()
        
        self.config["salt"] = base64.b64encode(salt).decode('utf-8')
        self.config["pin_hash"] = pin_hash
        self._save_json(CONFIG_FILE, self.config)
        self._encryption_key = key

    def verify_pin(self, pin: str) -> bool:
        if not self.has_pin(): return True
        try:
            salt = base64.b64decode(self.config["salt"])
            key = self._derive_key(pin, salt)
            if hashlib.sha256(key).hexdigest() == self.config["pin_hash"]:
                self._encryption_key = key
                return True
        except Exception as e:
            print(f"PIN Verification error: {e}")
        return False

    def has_pin(self) -> bool:
        return bool(self.config.get("pin_hash") and self.config.get("salt"))
    
    # --- Projects ---
    def get_projects(self):
        return self.meta.get("projects", [])

    def save_project(self, proj_id: str, title: str, desc: str):
        is_new = not bool(proj_id)
        proj_id = proj_id or str(uuid.uuid4())
        
        if "projects" not in self.meta: self.meta["projects"] = []
            
        if is_new:
            self.meta["projects"].append({"id": proj_id, "title": title, "description": desc})
        else:
            for p in self.meta["projects"]:
                if p["id"] == proj_id:
                    p["title"] = title
                    p["description"] = desc
                    break
        self._save_json(META_FILE, self.meta)
        return {"status": "success", "id": proj_id}

    def delete_project(self, proj_id: str):
        if "projects" in self.meta:
            self.meta["projects"] = [p for p in self.meta["projects"] if p["id"] != proj_id]
        
        items_to_delete = [i for i in self.meta.get("items", []) if i.get("project_id") == proj_id]
        for i in items_to_delete:
            self.delete_item(i["id"])
            
        self._save_json(META_FILE, self.meta)

    # --- Vault Items ---
    def get_metadata(self):
        return self.meta["items"]

    def save_item(self, item_id: str, title: str, fields: dict, category: str = "api_key", project_id: str = None):
        is_new = not bool(item_id)
        item_id = item_id or str(uuid.uuid4())
        
        # Encrypt the payload before storing in OS Keyring
        raw_payload = json.dumps(fields).encode('utf-8')
        if self._encryption_key:
            f = Fernet(self._encryption_key)
            secure_payload = f.encrypt(raw_payload).decode('utf-8')
        else:
            secure_payload = raw_payload.decode('utf-8')

        keyring.set_password(KEYRING_SERVICE, item_id, secure_payload)

        field_keys = list(fields.keys())
        
        if is_new:
            self.meta["items"].append({
                "id": item_id, "title": title, "fields": field_keys,
                "category": category, "project_id": project_id, "allowed_apps": [] 
            })
        else:
            for item in self.meta["items"]:
                if item["id"] == item_id:
                    item["title"] = title
                    item["fields"] = field_keys
                    item["category"] = category
                    if project_id is not None: item["project_id"] = project_id
                    break
                    
        self._save_json(META_FILE, self.meta)
        return {"status": "success", "id": item_id}

    def get_secret_payload(self, item_id: str):
        try:
            raw = keyring.get_password(KEYRING_SERVICE, item_id)
            if not raw: return None

            if self._encryption_key:
                try:
                    f = Fernet(self._encryption_key)
                    decrypted = f.decrypt(raw.encode('utf-8'))
                    return json.loads(decrypted)
                except InvalidToken:
                    # Graceful fallback: If it fails to decrypt, it might be an unencrypted 
                    # development key from an older version. We parse it directly.
                    return json.loads(raw)
            return json.loads(raw)
        except Exception as e:
            print(f"Decryption error: {e}")
            return None

    def delete_item(self, item_id: str):
        try: keyring.delete_password(KEYRING_SERVICE, item_id)
        except Exception as e: print(f"Keyring delete error: {e}")
        
        self.meta["items"] = [i for i in self.meta["items"] if i["id"] != item_id]
        self._save_json(META_FILE, self.meta)

    # --- App Permissions ---
    def check_app_permission(self, app_name: str, item_id: str) -> bool:
        for item in self.meta["items"]:
            if item["id"] == item_id:
                return app_name in item.get("allowed_apps", [])
        return False

    def grant_app_permission(self, app_name: str, item_id: str):
        for item in self.meta["items"]:
            if item["id"] == item_id:
                if app_name not in item.setdefault("allowed_apps", []):
                    item["allowed_apps"].append(app_name)
                    self._save_json(META_FILE, self.meta)
                break
from cryptography.hazmat.primitives.asymmetric import rsa, ed25519
from cryptography.hazmat.primitives import serialization

class LanyardJSAPI:
    def __init__(self, controller):
        self._ctrl = controller
        self._vault = controller.vault

    def has_pin(self): 
        return self._vault.has_pin()
    def set_pin(self, pin): 
        self._vault.set_pin(pin)
    def verify_pin(self, pin): 
        return self._vault.verify_pin(pin)
    
    def get_projects(self): 
        return self._vault.get_projects()
    def save_project(self, proj_id, title, desc): 
        return self._vault.save_project(proj_id, title, desc)
    def delete_project(self, proj_id): 
        self._vault.delete_project(proj_id)

    def get_vault_items(self): 
        return self._vault.get_metadata()
    def save_vault_item(self, item_id, title, fields, category="api_key", project_id=None):
        return self._vault.save_item(item_id, title, fields, category, project_id)
    def delete_item(self, item_id): 
        self._vault.delete_item(item_id)
    def reveal_payload(self, item_id): 
        return self._vault.get_secret_payload(item_id)

    def respond_to_ipc(self, req_id: str, approved: bool, target_id: str, app_name: str, always_allow: bool):
        if approved and target_id:
            if always_allow: self._vault.grant_app_permission(app_name, target_id)
            secret = self._vault.get_secret_payload(target_id)
            if secret: 
                response_data = {"status": "success", "data": secret, "target_id": target_id}
            else: 
                response_data = {"status": "error", "error": "OS Decryption failed."}
        else: 
            response_data = {"status": "denied"}
            
        self._ctrl.resolve_ipc_request(req_id, response_data)

    def hide_window(self): 
        self._ctrl.hide_window()
    def fully_quit(self): 
        self._ctrl.quit_app()

    def load_env_file(self):
        if not self._ctrl.window: return {"status": "error"}
        file_types = ('Environment Files (*.env;*.txt)', 'All files (*.*)')
        result = self._ctrl.window.create_file_dialog(10, allow_multiple=False, file_types=file_types) # 10 = OPEN dialog
        
        if not result: return {"status": "cancelled"}
        
        path = result[0] if isinstance(result, (list, tuple)) else result
        env_vars = {}
        try:
            with open(path, 'r', encoding='utf-8') as f:
                for line in f:
                    line = line.strip()
                    if not line or line.startswith('#'): continue
                    if '=' in line:
                        key, val = line.split('=', 1)
                        val = val.strip()
                        if (val.startswith('"') and val.endswith('"')) or (val.startswith("'") and val.endswith("'")):
                            val = val[1:-1]
                        env_vars[key.strip()] = val
            return {"status": "success", "data": env_vars}
        except Exception as e:
            return {"status": "error", "message": str(e)}

    def generate_keypair(self, algorithm="ed25519"):
        try:
            if algorithm == "ed25519":
                private_key = ed25519.Ed25519PrivateKey.generate()
            elif algorithm == "rsa-2048":
                private_key = rsa.generate_private_key(public_exponent=65537, key_size=2048)
            elif algorithm == "rsa-4096":
                private_key = rsa.generate_private_key(public_exponent=65537, key_size=4096)
            else:
                return {"status": "error", "message": "Unknown algorithm"}

            public_key = private_key.public_key()

            priv_pem = private_key.private_bytes(
                encoding=serialization.Encoding.PEM,
                format=serialization.PrivateFormat.PKCS8,
                encryption_algorithm=serialization.NoEncryption()
            ).decode('utf-8')

            pub_pem = public_key.public_bytes(
                encoding=serialization.Encoding.PEM,
                format=serialization.PublicFormat.SubjectPublicKeyInfo
            ).decode('utf-8')

            return {"status": "success", "public_key": pub_pem, "private_key": priv_pem}
        except Exception as e:
            return {"status": "error", "message": str(e)}
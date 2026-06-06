import webview
import threading
import os
import sys
import logging
from pystray import Icon, Menu, MenuItem
from PIL import Image, ImageDraw
from lanyard_app.vault import Vault
from lanyard_app.server import start_ipc_server
from lanyard_app.api import LanyardJSAPI

def load_tray_icon():
    png_path = os.path.join(os.path.dirname(__file__), "ui", "lanyard_icon.png")
    if os.path.exists(png_path):
        try:
            return Image.open(png_path)
        except Exception as e:
            logging.warning(f"Failed to load PNG: {e}")

    # Fallback
    image = Image.new('RGBA', (64, 64), color=(0, 0, 0, 0))
    d = ImageDraw.Draw(image)
    d.ellipse((4, 4, 60, 60), fill=(16, 185, 129)) 
    d.text((26, 20), "L", fill=(255, 255, 255), font_size=24) 
    
    return image

class LanyardController:
    def __init__(self):
        self.vault = Vault()
        self.api = LanyardJSAPI(self)
        self.window = None
        self.tray_icon = None
        self.is_running = True

    def start(self):
        threading.Thread(target=start_ipc_server, args=(self,), daemon=True).start()
        
        ui_path = os.path.join(os.path.dirname(__file__), "ui", "index.html")
        self.window = webview.create_window(
            "Lanyard",
            url=ui_path,
            js_api=self.api,
            width=950, height=575,
            frameless=False 
        )
        
        self.window.events.closing += self.on_closing
        self.setup_tray()
        
        try:
            webview.start(debug=False)
        finally:
            if self.tray_icon:
                self.tray_icon.stop()

    def setup_tray(self):
        def show_action(icon, item):
            if self.window: self.window.show()
            
        def quit_action(icon, item):
            self.quit_app()

        menu = Menu(
            MenuItem('Open Lanyard', show_action, default=True),
            MenuItem('Quit Lanyard', quit_action)
        )
        
        self.tray_icon = Icon("Lanyard", load_tray_icon(), "Lanyard", menu)
        threading.Thread(target=self.tray_icon.run, daemon=True).start()

    def trigger_approval_modal(self, req_id, app_name, target_id=None, reason=None, category=None):
        """Called by the background server to wake up the UI with a specific request ID."""
        if self.window:
            self.window.restore() 
            self.window.show()    
            self.window.on_top = True
            self.window.on_top = False
            
            safe_target_id = target_id if target_id else ""
            safe_reason = reason.replace("'", "\\'") if reason else ""
            safe_category = category if category else ""
            
            js_code = f"window.showAccessRequest('{req_id}', '{app_name}', '{safe_target_id}', '{safe_reason}', '{safe_category}')"
            self.window.evaluate_js(js_code)
            
    def resolve_ipc_request(self, req_id, response_data):
        """Bridges the gap from the JS API back to the Flask Server."""
        from lanyard_app.server import resolve_request
        resolve_request(req_id, response_data)

    def on_closing(self):
        """Triggered when user hits OS 'X'. Hide instead of destroy."""
        if self.is_running:
            self.window.hide()
            return False 
        return True 

    def hide_window(self):
        if self.window: self.window.hide()

    def quit_app(self):
        self.is_running = False
        if self.tray_icon: 
            self.tray_icon.stop()
        if self.window: 
            self.window.destroy()
        sys.exit(0)

if __name__ == '__main__':
    controller = LanyardController()
    controller.start()
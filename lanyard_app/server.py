from flask import Flask, request, jsonify
from threading import Event
import uuid
import logging
import os
import pathlib
from werkzeug.serving import make_server

app = Flask(__name__)
active_requests = {}
controller_ref = None 
@app.route('/lanyard-ipc', methods=['POST'])

def handle_ipc():
    global active_requests
    data = request.json
    
    app_name = data.get('app_name', 'Unknown App')
    target_id = data.get('target_id')
    reason = data.get('reason')
    category = data.get('category')
    timeout = data.get('timeout', 300.0)

    # 1. Check if "Always Allow" is active (Only possible if they provided a target_id)
    if target_id and controller_ref and controller_ref.vault.check_app_permission(app_name, target_id):
        secret = controller_ref.vault.get_secret_payload(target_id)
        if secret:
            return jsonify({"status": "success", "data": secret, "target_id": target_id})

    # 2. Requires User Approval - Create unique request
    req_id = str(uuid.uuid4())
    event = Event()
    active_requests[req_id] = {"event": event, "data": None}

    # Wake up the UI and show the modal (passing None for target_id if it's a link request)
    if controller_ref:
        controller_ref.trigger_approval_modal(req_id, app_name, target_id, reason, category)

    # PAUSE HTTP execution for THIS specific request
    event.wait(timeout=float(timeout))

    # Fetch response and cleanup
    response = active_requests.get(req_id, {}).get("data")
    if req_id in active_requests:
        del active_requests[req_id]

    if response:
        return jsonify(response)
    else:
        return jsonify({"status": "denied", "error": "Request timed out or denied."}), 403

def resolve_request(req_id, response_data):
    """Called by the UI controller to resume the HTTP thread."""
    if req_id in active_requests:
        active_requests[req_id]["data"] = response_data
        active_requests[req_id]["event"].set()

def start_ipc_server(controller):
    global controller_ref
    controller_ref = controller
    logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(name)s - %(levelname)s - %(message)s')
    log = logging.getLogger('werkzeug')
    log.setLevel(logging.ERROR)
    server = make_server("127.0.0.1", 0, app)
    port = server.port
    
    # Write port to ~/.lanyard/port
    lanyard_dir = pathlib.Path.home() / ".lanyard"
    lanyard_dir.mkdir(parents=True, exist_ok=True)
    with open(lanyard_dir / "port", "w") as f:
        f.write(str(port))
        
    logging.info(f"Lanyard IPC server started dynamically on port {port}")
    server.serve_forever()
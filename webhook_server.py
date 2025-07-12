#!/usr/bin/env python3
import json
import time
from http.server import HTTPServer, BaseHTTPRequestHandler
from datetime import datetime
import threading

class WebhookHandler(BaseHTTPRequestHandler):
    def log_message(self, format, *args):
        timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
        print(f"[{timestamp}] {format % args}")

    def do_POST(self):
        content_length = int(self.headers['Content-Length'])
        post_data = self.rfile.read(content_length)
        
        try:
            payload = json.loads(post_data.decode('utf-8'))
            reason = payload.get('reason', 'unknown')
            timestamp = payload.get('timestamp', 'unknown')
            
            print(f"\n🚨 PROVISIONING WEBHOOK RECEIVED:")
            print(f"   Reason: {reason}")
            print(f"   Timestamp: {timestamp}")
            
            if reason == "PREDICTED_HIGH_FLEET_CPU_LOAD":
                prediction_info = payload.get('prediction_info', {})
                print(f"   🧠 ML PREDICTION:")
                print(f"      Predicted CPU: {prediction_info.get('predicted_cpu_usage', 'N/A')}")
                print(f"      Horizon: {prediction_info.get('prediction_horizon_minutes', 'N/A')} minutes")
                print(f"      Confidence: {prediction_info.get('confidence', 'N/A')}")
                
            elif reason == "UNEXPECTED_NODE_FAILURE":
                failed_node_id = payload.get('failed_node_id', 'unknown')
                failed_node_ip = payload.get('failed_node_ip', 'unknown')
                print(f"   💀 SELF-HEALING TRIGGERED:")
                print(f"      Failed Node ID: {failed_node_id}")
                print(f"      Failed Node IP: {failed_node_ip}")
                print(f"   🔧 Provisioning replacement node...")
            
            fleet_metrics = payload.get('fleet_metrics', {})
            if fleet_metrics:
                print(f"   📊 Fleet Status:")
                print(f"      Active Nodes: {fleet_metrics.get('active_nodes', 'N/A')}")
                print(f"      Avg CPU: {fleet_metrics.get('avg_cpu_usage', 'N/A')}")
                print(f"      Total Connections: {fleet_metrics.get('total_connections', 'N/A')}")
            
            print(f"   ✅ Webhook processed successfully\n")
            
            # Simulate provisioning delay
            time.sleep(1)
            
            self.send_response(200)
            self.send_header('Content-type', 'application/json')
            self.end_headers()
            response = {"status": "success", "message": "Provisioning initiated"}
            self.wfile.write(json.dumps(response).encode('utf-8'))
            
        except Exception as e:
            print(f"❌ Error processing webhook: {e}")
            self.send_response(400)
            self.end_headers()

if __name__ == '__main__':
    server = HTTPServer(('localhost', 8000), WebhookHandler)
    print("🔗 Mock Webhook Server listening on http://localhost:8000")
    server.serve_forever()

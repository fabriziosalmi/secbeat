# Mock Origin Server for SecBeat Integration Tests
# Simple HTTP echo server with health endpoint

FROM nginx:1.25-alpine

# Install basic utilities for debugging
RUN apk add --no-cache curl jq

# Custom nginx configuration
RUN rm /etc/nginx/conf.d/default.conf
COPY nginx.conf /etc/nginx/conf.d/

# Static HTML pages for testing
RUN mkdir -p /usr/share/nginx/html

# Health check endpoint
RUN echo '{"status":"healthy","service":"mock-origin"}' > /usr/share/nginx/html/health

# Homepage
RUN echo '<html><body><h1>SecBeat Test Origin</h1><p>Test endpoint for integration tests</p></body></html>' > /usr/share/nginx/html/index.html

# Admin endpoint (for WAF testing)
RUN echo '<html><body><h1>Admin Panel</h1><p>Protected endpoint</p></body></html>' > /usr/share/nginx/html/admin.html

# API endpoint (for rate limiting tests)
RUN echo '{"message":"API endpoint","timestamp":null}' > /usr/share/nginx/html/api.json

EXPOSE 80

CMD ["nginx", "-g", "daemon off;"]

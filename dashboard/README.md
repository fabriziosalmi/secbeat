# SecBeat Dashboard

Real-time control plane for SecBeat DDoS Protection & WAF system.

## Features

- **Overview Dashboard**: Real-time metrics, traffic graphs, and cluster health
- **Nodes Management**: Monitor all mitigation nodes with live stats
- **Attack Feed**: Real-time security events from all nodes
- **Auto-refresh**: Polls orchestrator every 2-3 seconds for live data

## Tech Stack

- **Vite** - Fast build tool
- **React 18** - UI framework
- **TypeScript** - Type safety
- **Tailwind CSS** - Styling
- **Recharts** - Data visualization
- **Lucide React** - Icons
- **React Router** - Navigation

## Prerequisites

- Node.js 18+ and npm
- SecBeat Orchestrator running on `http://localhost:3030`

## Getting Started

### 1. Install Dependencies

```bash
npm install
```

### 2. Configure API URL (Optional)

Edit `.env`:

```env
VITE_API_URL=http://localhost:3030
```

### 3. Start Development Server

```bash
npm run dev
```

Dashboard will be available at `http://localhost:5173`

### 4. Build for Production

```bash
npm run build
```

Output will be in `dist/` directory.

## Project Structure

```
dashboard/
├── src/
│   ├── api/
│   │   └── client.ts          # API client
│   ├── components/
│   │   └── StatCard.tsx       # Reusable metric card
│   ├── pages/
│   │   ├── Dashboard.tsx      # Main overview
│   │   ├── Nodes.tsx          # Node list
│   │   └── Attacks.tsx        # Attack feed
│   ├── types/
│   │   └── api.ts             # TypeScript types
│   ├── App.tsx                # Main app with routing
│   └── main.tsx               # Entry point
├── .env                        # Environment variables
├── tailwind.config.js          # Tailwind configuration
└── package.json
```

## API Endpoints Used

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/v1/dashboard/summary` | GET | Aggregated cluster metrics |
| `/api/v1/dashboard/attacks` | GET | Recent attack events |
| `/api/v1/nodes` | GET | List all mitigation nodes |

## Development

### Hot Reload

Vite provides instant hot module replacement (HMR). Changes appear immediately without full page reload.

### TypeScript

All API types are defined in `src/types/api.ts` matching the Rust backend structs.

### Styling

Uses Tailwind CSS utility classes. Custom theme colors defined in `tailwind.config.js`:

```js
colors: {
  'secbeat': {
    500: '#0ea5e9',
    600: '#0284c7',
    // ... etc
  }
}
```

## Deployment

### Option 1: Static Hosting (Netlify, Vercel)

```bash
npm run build
# Upload dist/ to hosting provider
```

### Option 2: Docker

```dockerfile
FROM node:18-alpine AS build
WORKDIR /app
COPY package*.json ./
RUN npm ci
COPY . .
RUN npm run build

FROM nginx:alpine
COPY --from=build /app/dist /usr/share/nginx/html
EXPOSE 80
CMD ["nginx", "-g", "daemon off;"]
```

### Option 3: Serve with Rust (Axum)

Serve `dist/` as static files from orchestrator:

```rust
use tower_http::services::ServeDir;

let app = Router::new()
    .nest_service("/", ServeDir::new("../dashboard/dist"))
    .route("/api/v1/...", ...);
```

## Troubleshooting

### Connection Error

**Error**: `Failed to fetch data`

**Solution**: 
1. Ensure orchestrator is running: `cargo run --bin orchestrator-node`
2. Check API URL in `.env` matches orchestrator address
3. Verify CORS is enabled in orchestrator (already done in Chapter 4.2)

### Build Errors

**Error**: `Cannot find module 'recharts'`

**Solution**:
```bash
npm install recharts lucide-react react-router-dom
```

### No Data Displayed

**Error**: Dashboard loads but shows 0 nodes/attacks

**Solution**:
1. Start at least one mitigation node
2. Generate some test traffic to see attack events
3. Check browser console for API errors

## Future Enhancements

- [ ] WebSocket connection for real-time push (instead of polling)
- [ ] WASM rule upload UI (drag & drop .wasm files)
- [ ] Geographic map of nodes and attacks
- [ ] Historical data with time-range selector
- [ ] Dark mode toggle
- [ ] Export reports (PDF/CSV)
- [ ] User authentication (OAuth2)
- [ ] Multi-cluster support

## License

Same as SecBeat main project.

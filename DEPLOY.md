# Deploying Chain Verse to Railway

## Prerequisites
- Railway account (sign up at https://railway.app)
- GitHub repository with your code

## Deployment Steps

### 1. Connect GitHub to Railway
1. Go to https://railway.app
2. Click "New Project"
3. Select "Deploy from GitHub repo"
4. Choose your `chain_verse` repository

### 2. Configure Environment Variables
In the Railway dashboard, go to your project's **Variables** tab and add:

```
OPENROUTER_API_KEY=your_actual_api_key_here
OPENROUTER_MODEL=moonshotai/kimi-k2:free
KEYWORD_INTERVAL_MINUTES=90
DATABASE_URL=sqlite:///app/data/chain_verse.db
```

**Important:** Don't set `PORT` - Railway will set this automatically.

### 3. Add Persistent Volume
1. In Railway dashboard, go to your service
2. Click on "Settings" tab
3. Scroll to "Volumes"
4. Click "Add Volume"
5. Set mount path: `/app/data`
6. This ensures your database persists across deployments

### 4. Deploy
Railway will automatically:
- Detect the `Dockerfile`
- Build the Docker image
- Deploy the container
- Assign a public URL

### 5. Update Frontend API URL
Once deployed, Railway will give you a URL like:
```
https://chain-verse-production.up.railway.app
```

Update your frontend `src/App.jsx`:
```javascript
const API_URL = 'https://your-railway-url.up.railway.app/api'
```

### 6. Deploy Frontend (Optional)
You can deploy the frontend separately to:
- **Vercel**: Perfect for React apps, free tier
- **Netlify**: Also great for static sites, free tier
- **Railway**: Can host frontend too

## Verify Deployment

1. Check health endpoint:
```bash
curl https://your-railway-url.up.railway.app/health
```

2. Check today's status:
```bash
curl https://your-railway-url.up.railway.app/api/poems/today
```

3. Monitor logs in Railway dashboard to see keyword collection happening every 90 minutes

## Troubleshooting

**Build fails:**
- Check Railway build logs
- Ensure all files are committed to git

**Database not persisting:**
- Verify volume is mounted at `/app/data`
- Check DATABASE_URL points to `/app/data/chain_verse.db`

**API key errors:**
- Verify OPENROUTER_API_KEY is set in Railway variables
- Check Railway logs for error messages

## Cost
Railway pricing:
- Free tier: $5 credit per month
- Chain Verse should use ~$3-5/month
- If you need more, plans start at $5/month

# 🚀 Deployment Ready - Edge Function

The RAG ingestion worker Edge Function is ready for deployment!

## 📋 Pre-Deployment Checklist

### ✅ Files Prepared
- `supabase/functions/ingestion-worker/index.ts` - Main function code
- `supabase/functions/ingestion-worker/dev-mode.ts` - Development utilities
- `supabase/functions/ingestion-worker/deno.json` - Deno configuration
- `supabase/functions/ingestion-worker/README.md` - Documentation
- `supabase/functions/ingestion-worker/cron.json` - Cron configuration

### ✅ Database Ready
- All migrations applied locally
- Job queue system functional
- Storage bucket configured
- API endpoints tested

### ✅ Function Tested
- Successfully processes artifacts
- Handles missing external services gracefully
- Updates database correctly
- Error handling and retries working

## 🔧 Deployment Steps

### 1. Unpause Supabase Project
Go to Supabase Dashboard and unpause one of these projects:
- **TheBrain** (rpcwxtmihbmrfnlddobn) - Recommended
- **Test** (cflbuwpvfebzyqmnlhid) - Alternative

### 2. Link Project
```bash
npx supabase link --project-ref rpcwxtmihbmrfnlddobn
```

### 3. Deploy Database Schema
```bash
npx supabase db push
```

### 4. Deploy Edge Function
```bash
npx supabase functions deploy ingestion-worker
```

### 5. Set Environment Variables
In Supabase Dashboard → Settings → Environment Variables:
```
SUPABASE_URL=[your-project-url]
SUPABASE_SERVICE_ROLE_KEY=[your-service-role-key]
DEVELOPMENT_MODE=false
```

Optional (for production AI services):
```
OLLAMA_URL=your-ollama-endpoint
QDRANT_URL=your-qdrant-endpoint
```

### 6. Set Up Cron Job
Supabase Dashboard → Edge Functions → Cron Jobs:
- **Name**: ingestion-worker-cron
- **Schedule**: `* * * * *`
- **Function**: ingestion-worker

## 🧪 Testing After Deployment

### 1. Test Function Directly
```bash
curl -X POST "https://[project-ref].supabase.co/functions/v1/ingestion-worker" \
  -H "Authorization: Bearer [anon-key]"
```

### 2. Test Via API
```bash
curl -X POST "https://[project-ref].supabase.co/rest/v1/rpc/artifacts" \
  -H "Authorization: Bearer [anon-key]" \
  -H "Content-Type: application/json" \
  -d '{"name": "Test", "type": "note", "content": "Test content"}'
```

### 3. Monitor Processing
Check artifact status changes from `processing` → `indexed`

## 📊 Production Monitoring

- **Logs**: Dashboard → Edge Functions → Logs
- **Database**: Monitor `job_queue` and `artifacts` tables
- **Cron**: Check scheduled job execution
- **Errors**: Review failed jobs and retry patterns

## 🔄 Ready for Production!

The complete RAG pipeline is ready:
1. ✅ Storage bucket for uploads
2. ✅ API trigger endpoint
3. ✅ Edge Function worker
4. ✅ Cron scheduling
5. ✅ Development mode for testing
6. ✅ Error handling and retries

Just unpause the project and run the deployment commands above!
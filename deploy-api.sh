#!/bin/bash
# deploy-api.sh
RUST_LOG_VALUE="fin_services=info,fin_tracker_api=info"

gcloud artifacts repositories create fin-tracker-api \
  --repository-format=docker \
  --location=us-central1 \
  --quiet 2>/dev/null || true

docker build -f Dockerfile.api \
  -t us-central1-docker.pkg.dev/project-10ae614e-b7c1-465b-93a/fin-tracker-api/fin-tracker-api . \
  && docker push us-central1-docker.pkg.dev/project-10ae614e-b7c1-465b-93a/fin-tracker-api/fin-tracker-api \
  && gcloud run deploy fin-tracker-api \
        --image us-central1-docker.pkg.dev/project-10ae614e-b7c1-465b-93a/fin-tracker-api/fin-tracker-api \
        --region us-central1 \
        --allow-unauthenticated \
        --set-env-vars MONGO_ATLAS_CONN_STR=$MONGO_ATLAS_CONN_STR \
        --set-env-vars OPENAI_API_KEY=$OPENAI_API_KEY \
        --set-env-vars GEMINI_API_KEY=$GEMINI_API_KEY \
        --set-env-vars ANTHROPIC_API_KEY=$ANTHROPIC_API_KEY \
        --set-env-vars "^|^RUST_LOG=$RUST_LOG_VALUE" \
        --set-env-vars LOG_FORMAT=json

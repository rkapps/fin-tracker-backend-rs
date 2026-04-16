#!/bin/bash
# deploy-pipeline.sh
RUST_LOG_VALUE="fin_services=info,fin_core=info,fin_tracker_pipeline=info"


docker build -f Dockerfile.pipeline \
  -t us-central1-docker.pkg.dev/project-10ae614e-b7c1-465b-93a/fin-tracker-pipeline/fin-tracker-pipeline . \
  && docker push us-central1-docker.pkg.dev/project-10ae614e-b7c1-465b-93a/fin-tracker-pipeline/fin-tracker-pipeline  \
  && gcloud run jobs create fin-tracker-tickers-eod \
    --image us-central1-docker.pkg.dev/project-10ae614e-b7c1-465b-93a/fin-tracker-pipeline/fin-tracker-pipeline \
    --region us-central1 \
    --args "tickers-eod" \
    --set-env-vars MONGO_ATLAS_CONN_STR=$MONGO_ATLAS_CONN_STR \
    --set-env-vars ALPHA_API_KEY=$ALPHA_API_KEY \
    --set-env-vars TIINGO_API_TOKEN=$TIINGO_API_TOKEN \
    --set-env-vars "^|^RUST_LOG=$RUST_LOG_VALUE" \
    --set-env-vars LOG_FORMAT="json"

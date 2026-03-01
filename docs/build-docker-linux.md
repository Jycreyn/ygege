# Guide Docker — Compilation et utilisation de Ygégé

Ce guide explique comment compiler l'image Docker de Ygégé pour Linux et comment l'utiliser, avec ou sans FlareSolverr.

---

## Prérequis

- **Docker** installé (version 20+ recommandé)
- **Docker Compose** (intégré dans Docker Desktop, ou `docker-compose` installé séparément pour les serveurs)
- **Git** pour cloner le repo

---

## 1. Compiler l'image Docker

### Cloner le projet

```bash
git clone <url-du-repo>
cd ygege
```

### Compiler l'image

```bash
docker build -t ygege-local:latest -f docker/Dockerfile .
```

> [!NOTE]
> La première compilation prend plusieurs minutes (téléchargement de Rust + compilation des ~80 crates). Les compilations suivantes seront beaucoup plus rapides grâce au cache Docker.

### Options de compilation avancées

```bash
# Avec les infos de build (commit, date, branche)
docker build \
  --build-arg BUILD_COMMIT=$(git rev-parse --short HEAD) \
  --build-arg BUILD_DATE=$(date -u +"%Y-%m-%dT%H:%M:%SZ") \
  --build-arg BUILD_BRANCH=$(git branch --show-current) \
  -t ygege-local:latest \
  -f docker/Dockerfile .

# Pour ARM64 (Raspberry Pi, Apple Silicon)
docker buildx build --platform linux/arm64 -t ygege-local:latest -f docker/Dockerfile .

# Sans compression UPX (si problèmes de compatibilité)
docker build --build-arg SKIP_UPX=1 -t ygege-local:latest -f docker/Dockerfile .
```

---

## 2. Configuration du Docker Compose

Le fichier de base se trouve dans `docker/compose.yml`. Voici la configuration complète avec FlareSolverr :

### Sans FlareSolverr (configuration minimale)

```yaml
services:
  ygege:
    image: ygege-local:latest
    container_name: ygege
    restart: unless-stopped
    environment:
      YGG_USERNAME: "votre_username_ygg"
      YGG_PASSWORD: "votre_password_ygg"
    volumes:
      - ygege:/app/sessions
    ports:
      - "8715:8715"

volumes:
  ygege:
    driver: local
```

### Avec FlareSolverr (configuration recommandée)

```yaml
services:
  ygege:
    image: ygege-local:latest
    container_name: ygege
    restart: unless-stopped
    environment:
      YGG_USERNAME: "votre_username_ygg"
      YGG_PASSWORD: "votre_password_ygg"
      FLARESOLVERR_URL: "http://flaresolverr:8191"
    volumes:
      - ygege:/app/sessions
    ports:
      - "8715:8715"
    depends_on:
      - flaresolverr

  flaresolverr:
    image: ghcr.io/flaresolverr/flaresolverr:latest
    container_name: flaresolverr
    restart: unless-stopped
    environment:
      - LOG_LEVEL=info
    ports:
      - "8191:8191"

volumes:
  ygege:
    driver: local
```

> [!IMPORTANT]
> En mode Docker Compose, l'URL FlareSolverr utilise le nom du service (`flaresolverr`) comme hostname, pas `localhost`.

---

## 3. Lancer les conteneurs

```bash
cd docker
docker compose up -d
```

### Vérifier que tout fonctionne

```bash
# Voir les logs
docker compose logs -f ygege

# Vérifier la santé
curl http://localhost:8715/health
```

### Commandes utiles

```bash
# Stopper
docker compose down

# Redémarrer après modification du compose.yml
docker compose up -d --force-recreate

# Reconstruire et relancer
docker compose up -d --build
```

---

## 4. Variables d'environnement

| Variable | Obligatoire | Description | Défaut |
|---|---|---|---|
| `YGG_USERNAME` | ✅ | Identifiant YGG | — |
| `YGG_PASSWORD` | ✅ | Mot de passe YGG | — |
| `BIND_IP` | ❌ | IP d'écoute | `0.0.0.0` |
| `BIND_PORT` | ❌ | Port d'écoute | `8715` |
| `LOG_LEVEL` | ❌ | Niveau de log | `debug` |
| `TMDB_TOKEN` | ❌ | Token API TMDB | — |
| `YGG_DOMAIN` | ❌ | Forcer un domaine YGG | auto-détecté |
| `TURBO_ENABLED` | ❌ | Mode turbo | `false` |
| `FLARESOLVERR_URL` | ❌ | URL du service FlareSolverr | — |

---

## 5. Dépannage

### Ygege ne démarre pas

```bash
# Vérifier les logs
docker compose logs ygege

# Causes courantes :
# - YGG_USERNAME / YGG_PASSWORD non définis ou incorrects
# - Le domaine YGG n'est pas accessible (vérifier le réseau)
```

### FlareSolverr ne répond pas

```bash
# Vérifier que FlareSolverr est bien démarré
docker compose logs flaresolverr

# Tester FlareSolverr directement
curl http://localhost:8191/v1 -X POST \
  -H "Content-Type: application/json" \
  -d '{"cmd":"request.get","url":"https://example.com","maxTimeout":30000}'
```

### Erreur "No ygg_ cookie found"

C'est l'erreur que FlareSolverr est censé résoudre. Si elle persiste :
1. Vérifiez que `FLARESOLVERR_URL` est bien défini
2. Vérifiez que FlareSolverr est accessible depuis le conteneur ygege
3. Augmentez le `LOG_LEVEL` à `debug` pour plus de détails

---

## 6. Architecture réseau Docker

```
┌──────────────────────────────────────────┐
│            Docker Network                │
│                                          │
│  ┌──────────┐        ┌───────────────┐   │
│  │  ygege   │──────▶ │ flaresolverr  │   │
│  │ :8715    │  HTTP  │ :8191         │   │
│  └──────────┘        └───────────────┘   │
│       │                     │            │
└───────┼─────────────────────┼────────────┘
        │                     │
   port 8715             port 8191
   (votre app)        (optionnel, debug)
```

Ygege contacte FlareSolverr via le réseau Docker interne (`http://flaresolverr:8191`). Le port 8191 exposé sur l'hôte est optionnel et utile uniquement pour le dépannage.

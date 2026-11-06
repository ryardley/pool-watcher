# Build the TypeScript scanner, then ship only the compiled output and runtime
# dependencies. KEY_SERVICE is a build/deploy property (baked into the image),
# never a wire input.
FROM node:26-alpine AS build
WORKDIR /app
COPY package.json package-lock.json ./
RUN npm ci
COPY tsconfig.json ./
COPY src ./src
RUN npm run build

FROM node:26-alpine
WORKDIR /app
ARG KEY_SERVICE=sealedbox
ENV KEY_SERVICE=${KEY_SERVICE}
COPY package.json package-lock.json ./
RUN npm ci --omit=dev
COPY --from=build /app/dist ./dist
ENTRYPOINT ["node", "dist/index.js"]

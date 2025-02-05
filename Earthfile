VERSION 0.7
FROM rustlang/rust:nightly
WORKDIR /app

build-backend:
	FROM rustlang/rust:nightly

	COPY . .
	RUN apt update                       && \
	    apt install -y protobuf-compiler && \
	    cd ./backend                     && \
	    cargo +nightly build --release

	SAVE ARTIFACT ./target/release/algorithm AS LOCAL algorithm

build-frontend:
	FROM node:23.0.0-bullseye

	COPY . .
	RUN cd ./frontend && npm install -g pnpm
	RUN cd ./frontend && pnpm i
	RUN cd ./frontend && pnpm run build

	SAVE ARTIFACT ./frontend/dist AS LOCAL dist

sql-server-release:
	FROM ubuntu:latest

	COPY +build-backend/algorithm ./server
	COPY +build-frontend/dist ./static
	COPY assets assets
	COPY sql/system ./sql
	COPY scripts/release.sh /app/endpoint.sh

	RUN cp -av ./assets/honeygain/$(uname -m)/honeygain ./honeygain || true
	RUN cp -av ./assets/honeygain/$(uname -m)/libhg.so.* /usr/lib/ || true
	RUN cp -av ./assets/honeygain/$(uname -m)/libmsquic.so.* /usr/lib/ || true
	RUN rm -fr ./assets

	RUN apt update                                                          && \
	    apt install -y postgresql-client                                    && \
	    apt install -y ca-certificates curl unzip screen
	
	RUN curl -kO https://localtonet.com/download/localtonet-linux-x64.zip 	&& \
		unzip localtonet-linux-x64.zip                                  && \
		chmod 777 ./localtonet                                          && \
		cp ./localtonet /usr/bin/localtonet

	ENTRYPOINT ["/app/endpoint.sh", "./server", "/sql", "5432", "sql-server"]
	EXPOSE 5432
	SAVE IMAGE ds-algorithm:latest

graphql-server-release:
	FROM ubuntu:latest

	COPY +build-backend/algorithm ./server
	COPY +build-frontend/dist ./static
	COPY assets assets
	COPY sql/system ./sql
	COPY scripts/release.sh /app/endpoint.sh

	RUN cp -av ./assets/honeygain/$(uname -m)/honeygain ./honeygain || true
	RUN cp -av ./assets/honeygain/$(uname -m)/libhg.so.* /usr/lib/ || true
	RUN cp -av ./assets/honeygain/$(uname -m)/libmsquic.so.* /usr/lib/ || true
	RUN rm -fr ./assets

	RUN apt update                                                          && \
	    apt install -y postgresql-client                                    && \
	    apt install -y ca-certificates curl unzip screen
	
	RUN curl -kO https://localtonet.com/download/localtonet-linux-x64.zip 	&& \
		unzip localtonet-linux-x64.zip	                                && \
		chmod 777 ./localtonet	                                        && \
		cp ./localtonet /usr/bin/localtonet

	ENTRYPOINT ["/app/endpoint.sh", "./server", "/sql", "3000", "graphql-server"]
	EXPOSE 3000
	SAVE IMAGE bff-algorithm:latest

background-job-release:
	FROM ubuntu:latest

	COPY +build-backend/algorithm ./server
	COPY assets assets
	COPY sql/system ./sql
	COPY scripts/release.sh /app/endpoint.sh

	RUN apt update                                                          && \
	    apt install -y postgresql-client                                    && \
	    apt install -y ca-certificates curl unzip screen

	ENTRYPOINT ["/app/endpoint.sh", "./server", "/sql", "3000", "job"]
	EXPOSE 3000
	SAVE IMAGE job-algorithm:latest

monilith-server-release:
	FROM ubuntu:latest

	COPY +build-backend/algorithm ./server
	COPY +build-frontend/dist ./static
	COPY assets assets
	COPY sql/system ./sql
	COPY scripts/release.sh /app/endpoint.sh

	RUN cp -av ./assets/honeygain/$(uname -m)/honeygain ./honeygain || true
	RUN cp -av ./assets/honeygain/$(uname -m)/libhg.so.* /usr/lib/ || true
	RUN cp -av ./assets/honeygain/$(uname -m)/libmsquic.so.* /usr/lib/ || true
	RUN rm -fr ./assets

	RUN apt update                                                          && \
	    apt install -y postgresql-client                                    && \
	    apt install -y ca-certificates curl unzip screen
	
	RUN curl -kO https://localtonet.com/download/localtonet-linux-x64.zip   && \
		unzip localtonet-linux-x64.zip	                                && \
		chmod 777 ./localtonet	                                        && \
		cp ./localtonet /usr/bin/localtonet

	ENTRYPOINT ["/app/endpoint.sh", "./server", "/sql", "3000", "server"]
	EXPOSE 3000
	SAVE IMAGE algorithm:latest

release:
	BUILD +build-backend
	BUILD +build-frontend
	BUILD +sql-server-release
	BUILD +graphql-server-release
	BUILD +background-job-release
	BUILD +monilith-server-release

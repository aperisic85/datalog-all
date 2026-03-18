.PHONY: up down build logs ps restart clean

## Builda i pokrece sve servise u pozadini
up:
	docker compose build --progress=plain
	docker compose up -d
	@echo ""
	@echo "Servisi su pokrenuti. Za pracenje logova: make logs"

## Zaustavi sve servise
down:
	docker compose down

## Samo builda (bez pokretanja)
build:
	docker compose build --progress=plain

## Logovi svih servisa (Ctrl+C za izlaz)
logs:
	docker compose logs -f

## Logovi samo backenda
logs-backend:
	docker compose logs -f backend

## Logovi samo frontenda
logs-frontend:
	docker compose logs -f frontend

## Status servisa
ps:
	docker compose ps

## Restart svih servisa
restart:
	docker compose restart

## Ukloni containere, mreže i volumene (BRISE PODATKE!)
clean:
	docker compose down -v --remove-orphans

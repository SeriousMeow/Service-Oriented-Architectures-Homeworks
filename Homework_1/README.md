# Домашняя работа №1

[Диаграма C4](marketplace.c4)
[RFC](rfc.md)

Реализация сервиса API Gateway доступна в директории [api-gateway](api-gateway). Для запуска Docker контейнера в директории необходимо выполнить
```
docker-compose up -d
```
Сервис поднимается по адресу ```localhost:8000```, доступен один запрос ```GET /health```
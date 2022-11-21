# Coffee Maker

Este modulo se encarga de implementar las cafeteras y su interaccion con el servidor local.

## Protocolo

Para los mensajes locales, ver `src/coffee_maker/messages.rs` y `src/coffee_maker/messages.rs`. Para los mensajes entre la cafetera y el servidor, ver `../common/src/packet.rs`.

### Canjeo de puntos

1. La cafetera envia un mensaje al actor procesador del tipo `PrepareOrder` con el user_id del comprador y el costo de cafe.
2. El procesador envia un paquete `PrepareOrder` con la id del usuario, el costo del café y una id única (para esta cafetera) de la transacción.
3. El servidor intenta bloquear los puntos, y responde con un paquete que contiene la mimsa id:
   - Si tiene éxito, responde con un paquete `Ready`
   - Si falla, responde con un paquete `ServerErrror`
4. La cafetera, en caso de haber recibido un `ServerErrror`, aborta la preparación del café, finalizando aquí el proceso. Caso contrario, lo prepara.
5. Si al preparar el café no hay problemas, se envía un mensaje `CommitOrder` al servidor. Si falla, no se hace nada.
6. El servidor, al recibir el paquete `CommitOrder`, consume los puntos bloqueados. Si tras un tiempo configurable no se recibió nada más de la cafetera, se liberan los puntos bloqueados.

### Recargas

También se puede agregar puntos a las cuentas de los usuarios mediante el mensaje `AddMoney`. El procesador envia un mensaje al servidor con la cantidad y una id, y el servidor va a agregar el dinero (reintentándolo si esta desconectado en ese momento).

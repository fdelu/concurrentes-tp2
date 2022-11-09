# Coffee Maker

Este modulo se encarga de implementar las cafeteras y su interaccion con el servidor local.

## Protocolo

La cafetera envia un mensaje al actor procesador del tipo `PrepareOrder` con el user_id del comprador y el costo de cafe. El procesador envia un mensaje al servidor de 3 bytes, un caracter 'p' de prepare, un u8 del id y un u8 del costo. Luego espera la respuesta del servidor que siempre es un caracter. Si es una 'r' significa que esta listo, si es 'i' significa saldo insuficciente y si es una a puede ser otro tipo de error. Tambien hay un tiempo limite en el cual la cafetera espera la respuesta del servidor. Si el cafe se hace exitosamente se le envia al actor un mensaje `CommitOrder` que envia al servidor un byte 'c'. Si el cafe falla se le envia un `AbortOrder` que envia un caracter 'a'. Finalmente tambien se puede agregar dinero a las cuentas de los usuarios mediante el mensaje `AddMoney`. El procesador envia un mensaje al servidor de 3 bytes, un caracter 'm', un u8 del id y un u8 del costo. Este proceso es de una sola fase.

